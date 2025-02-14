// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This module will eventually be factored out into a separate crate.
//!
//! In the meantime, we intentionally don't make the types in this module part of
//! our public API, but still implement methods that a standalone crate would have.
//!
//! The types defined in this module don't *actually* implement an arena. They use
//! 100% safe code, which has a significant performance overhead. The final version
//! will use an arena and unsafe code, but should have the exact same exported API as
//! this module.

use hashbrown::HashMap;

use crate::NodeId;

#[derive(Debug)]
struct TreeNode<T> {
    id: NodeId,
    item: T,
    children: HashMap<NodeId, TreeNode<T>>,
}

/// A container type for a tree of items.
///
/// This type is used to store zero, one or many trees of a given item type. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
#[derive(Debug, Default)]
pub struct TreeArena<T> {
    roots: HashMap<NodeId, TreeNode<T>>,
    parents_map: HashMap<NodeId, Option<NodeId>>,
}

/// A reference type giving shared access to an arena item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an `ArenaRef`.
/// You can iterate over its children to get access to child `ArenaRef` handles.
#[derive(Debug)]
pub struct ArenaRef<'arena, T> {
    /// The parent of this node
    pub parent_id: Option<NodeId>,
    /// The payload of the node
    pub item: &'arena T,
    /// Reference to the children of the node
    pub children: ArenaRefList<'arena, T>,
}

/// A reference type giving mutable access to an arena item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an `ArenaMut`.
/// This struct holds three fields:
///  - the id of its parent.
///  - a reference to the item itself.
///  - an [`ArenaMutList`] handle to access its children.
///
/// Because the latter two are disjoint references, you can mutate the node's value
/// and its children independently without invalidating the references.
///
/// You can iterate over its children to get access to child `ArenaMut` handles.
#[derive(Debug)]
pub struct ArenaMut<'arena, T> {
    /// The parent of the node
    pub parent_id: Option<NodeId>,
    /// The payload of the node
    pub item: &'arena mut T,
    /// Reference to the children of the node
    pub children: ArenaMutList<'arena, T>,
}

/// A handle giving shared access to a set of arena items.
///
/// See [`ArenaRef`] for more information.
#[derive(Debug)]
pub struct ArenaRefList<'arena, T> {
    parent_id: Option<NodeId>,
    children: &'arena HashMap<NodeId, TreeNode<T>>,
    parents_map: ArenaMapRef<'arena>,
}

/// A handle giving mutable access to a set of arena items.
///
/// See [`ArenaMut`] for more information.
#[derive(Debug)]
pub struct ArenaMutList<'arena, T> {
    parent_id: Option<NodeId>,
    children: &'arena mut HashMap<NodeId, TreeNode<T>>,
    parents_map: ArenaMapMut<'arena>,
}

/// A shared reference to the parent father map
#[derive(Clone, Copy, Debug)]
pub struct ArenaMapRef<'arena> {
    parents_map: &'arena HashMap<NodeId, Option<NodeId>>,
}

/// A mutable reference to the parent father map
#[derive(Debug)]
pub struct ArenaMapMut<'arena> {
    parents_map: &'arena mut HashMap<NodeId, Option<NodeId>>,
}

// -- MARK: IMPLS ---

impl<T> Clone for ArenaRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArenaRef<'_, T> {}

impl<T> Clone for ArenaRefList<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArenaRefList<'_, T> {}

impl<T> TreeArena<T> {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self {
            roots: HashMap::new(),
            parents_map: HashMap::new(),
        }
    }

    /// Returns a handle giving access to the roots of the tree.
    pub fn roots(&self) -> ArenaRefList<'_, T> {
        ArenaRefList {
            parent_id: None,
            children: &self.roots,
            parents_map: ArenaMapRef {
                parents_map: &self.parents_map,
            },
        }
    }

    /// Returns a handle giving access to the roots of the tree.
    ///
    /// Using [`insert`](ArenaMutList::insert) on this handle
    /// will add a new root to the tree.
    pub fn roots_mut(&mut self) -> ArenaMutList<'_, T> {
        ArenaMutList {
            parent_id: None,
            children: &mut self.roots,
            parents_map: ArenaMapMut {
                parents_map: &mut self.parents_map,
            },
        }
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        self.roots().find_inner(id.into())
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        self.roots_mut().find_mut_inner(id.into())
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If the id is not in the tree, returns an empty vector.
    pub fn get_id_path(&self, id: impl Into<NodeId>) -> Vec<NodeId> {
        let parents_map = ArenaMapRef {
            parents_map: &self.parents_map,
        };
        parents_map.get_id_path(id.into(), None)
    }
}

impl<T> TreeNode<T> {
    fn arena_ref<'arena>(
        &'arena self,
        parent_id: Option<NodeId>,
        parents_map: &'arena HashMap<NodeId, Option<NodeId>>,
    ) -> ArenaRef<'arena, T> {
        ArenaRef {
            parent_id,
            item: &self.item,
            children: ArenaRefList {
                parent_id: Some(self.id),
                children: &self.children,
                parents_map: ArenaMapRef { parents_map },
            },
        }
    }

    fn arena_mut<'arena>(
        &'arena mut self,
        parent_id: Option<NodeId>,
        parents_map: &'arena mut HashMap<NodeId, Option<NodeId>>,
    ) -> ArenaMut<'arena, T> {
        ArenaMut {
            parent_id,
            item: &mut self.item,
            children: ArenaMutList {
                parent_id: Some(self.id),
                children: &mut self.children,
                parents_map: ArenaMapMut { parents_map },
            },
        }
    }
}

impl<T> ArenaRef<'_, T> {
    /// Id of the item this handle is associated with.
    pub fn id(&self) -> NodeId {
        self.children
            .parent_id
            .expect("ArenaRefList always has a parent_id when it's a member of ArenaRef")
    }
}

impl<T> ArenaMut<'_, T> {
    /// Id of the item this handle is associated with.
    pub fn id(&self) -> NodeId {
        self.children
            .parent_id
            .expect("ArenaRefList always has a parent_id when it's a member of ArenaRef")
    }

    /// Returns a shared reference equivalent to this one.
    pub fn reborrow(&mut self) -> ArenaRef<'_, T> {
        ArenaRef {
            parent_id: self.parent_id,
            item: self.item,
            children: self.children.reborrow(),
        }
    }

    /// Returns a mutable reference equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMut<'_, T> {
        ArenaMut {
            parent_id: self.parent_id,
            item: self.item,
            children: self.children.reborrow_mut(),
        }
    }
}

impl<'arena, T> ArenaRefList<'arena, T> {
    /// Returns true if the list has an element with the given id.
    pub fn has(self, id: impl Into<NodeId>) -> bool {
        let id = id.into();
        self.children.contains_key(&id)
    }

    /// Get a handle to the element of the list with the given id.
    pub fn item(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        self.children
            .get(&id)
            .map(|child| child.arena_ref(self.parent_id, self.parents_map.parents_map))
    }

    /// Get a handle to the element of the list with the given id.
    ///
    /// This is the same as [`item`](Self::item), except it consumes
    /// self. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        self.children
            .get(&id)
            .map(|child| child.arena_ref(self.parent_id, self.parents_map.parents_map))
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        self.find_inner(id.into())
    }

    fn find_inner(self, id: NodeId) -> Option<ArenaRef<'arena, T>> {
        let parent_id = self.parents_map.parents_map.get(&id)?;

        let id_path = if let Some(parent_id) = parent_id {
            self.parents_map.get_id_path(*parent_id, self.parent_id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children = self.children;
        while let Some((ancestor_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &node_children.get(ancestor_id)?.children;
        }

        let node = node_children.get(&id)?;
        Some(node.arena_ref(*parent_id, self.parents_map.parents_map))
    }
}

impl<'arena, T> ArenaMutList<'arena, T> {
    /// Returns true if the list has an element with the given id.
    pub fn has(self, id: impl Into<NodeId>) -> bool {
        let id = id.into();
        self.children.contains_key(&id)
    }

    /// Get a shared handle to the element of the list with the given id.
    pub fn item(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        self.children
            .get(&id)
            .map(|child| child.arena_ref(self.parent_id, self.parents_map.parents_map))
    }

    /// Get a mutable handle to the element of the list with the given id.
    pub fn item_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        let id = id.into();
        self.children
            .get_mut(&id)
            .map(|child| child.arena_mut(self.parent_id, self.parents_map.parents_map))
    }

    /// Get a shared handle to the element of the list with the given id.
    ///
    /// This is the same as [`item`](Self::item), except it consumes
    /// self. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        self.children
            .get(&id)
            .map(|child| child.arena_ref(self.parent_id, self.parents_map.parents_map))
    }

    /// Get a mutable handle to the element of the list with the given id.
    ///
    /// This is the same as [`item_mut`](Self::item_mut), except it consumes
    /// self. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        self.children
            .get_mut(&id)
            .map(|child| child.arena_mut(self.parent_id, self.parents_map.parents_map))
    }

    // TODO - Remove the child_id argument once creation of Widgets is figured out.
    // Return the id instead.
    // TODO - Add #[must_use]
    /// Insert a child into the tree under the common parent of this list's items.
    ///
    /// If this list was returned from [`TreeArena::roots_mut()`], create a new tree root.
    ///
    /// The new child will have the given id.
    ///
    /// Returns a handle to the new child.
    ///
    /// # Panics
    ///
    /// If the arena already contains an item with the given id.
    pub fn insert(&mut self, child_id: impl Into<NodeId>, value: T) -> ArenaMut<'_, T> {
        let child_id = child_id.into();
        assert!(
            !self.parents_map.parents_map.contains_key(&child_id),
            "Key already present"
        );
        self.parents_map
            .parents_map
            .insert(child_id, self.parent_id);

        self.children.insert(
            child_id,
            TreeNode {
                id: child_id,
                item: value,
                children: HashMap::new(),
            },
        );

        self.children
            .get_mut(&child_id)
            .unwrap()
            .arena_mut(self.parent_id, self.parents_map.parents_map)
    }

    // TODO - How to handle when a subtree is removed?
    // Move children to the root?
    /// Remove the item with the given id from the arena.
    ///
    /// If the id isn't in the list (even if it's e.g. a descendant), does nothing
    /// and returns None.
    ///
    /// Else, returns the removed item.
    ///
    /// This will also silently remove any recursive grandchildren of this item.
    #[must_use]
    pub fn remove(&mut self, child_id: impl Into<NodeId>) -> Option<T> {
        let child_id = child_id.into();
        let child = self.children.remove(&child_id)?;

        fn remove_children_from_map<I>(
            node: &TreeNode<I>,
            parents_map: &mut HashMap<NodeId, Option<NodeId>>,
        ) {
            for child in &node.children {
                remove_children_from_map(child.1, parents_map);
            }
            parents_map.remove(&node.id);
        }

        remove_children_from_map(&child, self.parents_map.parents_map);

        Some(child.item)
    }

    /// Returns a shared handle equivalent to this one.
    pub fn reborrow(&self) -> ArenaRefList<'_, T> {
        ArenaRefList {
            parent_id: self.parent_id,
            children: &*self.children,
            parents_map: self.parents_map.reborrow(),
        }
    }

    /// Returns a mutable handle equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutList<'_, T> {
        ArenaMutList {
            parent_id: self.parent_id,
            children: &mut *self.children,
            parents_map: self.parents_map.reborrow_mut(),
        }
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth).
    pub fn find(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        self.reborrow().find(id)
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a mutable reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth).
    pub fn find_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        self.find_mut_inner(id.into())
    }

    fn find_mut_inner(self, id: NodeId) -> Option<ArenaMut<'arena, T>> {
        let parent_id = self.parents_map.parents_map.get(&id)?;

        let id_path = if let Some(parent_id) = parent_id {
            self.parents_map.get_id_path(*parent_id, self.parent_id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children: &'arena mut _ = &mut *self.children;
        while let Some((ancestor_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &mut node_children.get_mut(ancestor_id)?.children;
        }

        let node = node_children.get_mut(&id)?;
        Some(node.arena_mut(*parent_id, &mut *self.parents_map.parents_map))
    }
}

impl ArenaMapRef<'_> {
    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If `start_id` is Some, the path ends just before that id instead; `start_id` is not included.
    ///
    /// If there is no path from `start_id` to id, returns an empty vector.
    pub fn get_id_path(self, id: NodeId, start_id: Option<NodeId>) -> Vec<NodeId> {
        let mut path = Vec::new();

        if !self.parents_map.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(current) = current_id {
            path.push(current);
            current_id = *self
                .parents_map
                .get(&current)
                .expect("All ids in the tree should have a parent in the parent map");
            if current_id == start_id {
                break;
            }
        }

        if current_id != start_id {
            path.clear();
        }

        path
    }
}

impl ArenaMapMut<'_> {
    /// Returns a shared handle equivalent to this one.
    pub fn reborrow(&self) -> ArenaMapRef<'_> {
        ArenaMapRef {
            parents_map: self.parents_map,
        }
    }

    /// Returns a mutable handle equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMapMut<'_> {
        ArenaMapMut {
            parents_map: self.parents_map,
        }
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If `start_id` is Some, the path ends just before that id instead; `start_id` is not included.
    ///
    /// If there is no path from `start_id` to id, returns an empty vector.
    pub fn get_id_path(&self, id: NodeId, start_id: Option<NodeId>) -> Vec<NodeId> {
        self.reborrow().get_id_path(id, start_id)
    }
}
