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

use super::NodeId;

use std::collections::HashMap;

#[derive(Debug)]
struct TreeNode<T> {
    id: NodeId,
    item: T,
    children: Vec<TreeNode<T>>,
}

// TODO - ArenaRefChildren and ArenaMutChildren might be easier to document if they were
// called ArenaRefGroup and ArenaMutGroup or something similar.

/// A container type for a tree of items.
///
/// This type is used to store zero, one or many trees of a given item type. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
#[derive(Debug, Default)]
pub struct TreeArena<T> {
    roots: Vec<TreeNode<T>>,
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
    pub children: ArenaRefChildren<'arena, T>,
}

/// A reference type giving mutable access to an arena item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an `ArenaMut`.
/// This struct holds three fields:
///  - the id of its parent.
///  - a reference to the item itself.
///  - an [`ArenaMutChildren`] handle to access its children.
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
    pub children: ArenaMutChildren<'arena, T>,
}

/// A handle giving shared access to an arena item's children.
///
/// See [`ArenaRef`] for more information.
#[derive(Debug)]
pub struct ArenaRefChildren<'arena, T> {
    id: Option<NodeId>,
    children: &'arena Vec<TreeNode<T>>,
    parents_map: ArenaMapRef<'arena>,
}

/// A handle giving mutable access to an arena item's children.
///
/// See [`ArenaMut`] for more information.
#[derive(Debug)]
pub struct ArenaMutChildren<'arena, T> {
    id: Option<NodeId>,
    children: &'arena mut Vec<TreeNode<T>>,
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

impl<T> Clone for ArenaRefChildren<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArenaRefChildren<'_, T> {}

impl<T> TreeArena<T> {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            parents_map: HashMap::new(),
        }
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    pub fn root_token(&self) -> ArenaRefChildren<'_, T> {
        ArenaRefChildren {
            id: None,
            children: &self.roots,
            parents_map: ArenaMapRef {
                parents_map: &self.parents_map,
            },
        }
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert_child`](ArenaMutChildren::insert_child) on this handle
    /// will add a new root to the tree.
    pub fn root_token_mut(&mut self) -> ArenaMutChildren<'_, T> {
        ArenaMutChildren {
            id: None,
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
        self.root_token().find_inner(id.into())
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        self.root_token_mut().find_mut_inner(id.into())
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
            children: ArenaRefChildren {
                id: Some(self.id),
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
            children: ArenaMutChildren {
                id: Some(self.id),
                children: &mut self.children,
                parents_map: ArenaMapMut { parents_map },
            },
        }
    }
}

impl<T> ArenaRef<'_, T> {
    /// Id of the item this handle is associated with.
    #[expect(
        clippy::missing_panics_doc,
        reason = "ArenaRefChildren always has an id when it's a member of ArenaRef"
    )]
    pub fn id(&self) -> NodeId {
        self.children.id.unwrap()
    }
}

impl<T> ArenaMut<'_, T> {
    /// Id of the item this handle is associated with.
    #[expect(
        clippy::missing_panics_doc,
        reason = "ArenaRefChildren always has an id when it's a member of ArenaRef"
    )]
    pub fn id(&self) -> NodeId {
        self.children.id.unwrap()
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

impl<'arena, T> ArenaRefChildren<'arena, T> {
    /// Returns true if the handle has a child with the given id.
    pub fn has_child(self, id: impl Into<NodeId>) -> bool {
        let id = id.into();
        self.children.iter().any(|child| child.id == id)
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a handle to access
    /// its children.
    pub fn get_child(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Find an arena item among descendants (this node not included).
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
            self.parents_map.get_id_path(*parent_id, self.id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children = self.children;
        while let Some((ancestor_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &node_children
                .iter()
                .find(|child| child.id == *ancestor_id)?
                .children;
        }

        let node = node_children.iter().find(|child| child.id == id)?;
        Some(node.arena_ref(*parent_id, self.parents_map.parents_map))
    }
}

impl<'arena, T> ArenaMutChildren<'arena, T> {
    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a handle to access
    /// its children.
    pub fn get_child(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a handle to access
    /// its children.
    pub fn get_child_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        let id = id.into();
        self.children
            .iter_mut()
            .find(|child| child.id == id)
            .map(|child| child.arena_mut(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child_mut`](Self::get_child_mut), except it consumes
    /// the handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        self.children
            .iter_mut()
            .find(|child| child.id == id)
            .map(|child| child.arena_mut(self.id, self.parents_map.parents_map))
    }

    // TODO - Remove the child_id argument once creation of Widgets is figured out.
    // Return the id instead.
    // TODO - Add #[must_use]
    /// Insert a child into the tree under the item associated with this handle.
    ///
    /// The new child will have the given id.
    ///
    /// # Panics
    ///
    /// The `insert_child` method will panic if the arena already contains a child
    /// with the given id.
    pub fn insert_child(&mut self, child_id: impl Into<NodeId>, value: T) {
        let child_id = child_id.into();
        assert!(
            !self.parents_map.parents_map.contains_key(&child_id),
            "Key already present"
        );
        self.parents_map.parents_map.insert(child_id, self.id);

        self.children.push(TreeNode {
            id: child_id,
            item: value,
            children: Vec::new(),
        });
    }

    // TODO - How to handle when a subtree is removed?
    // Move children to the root?
    /// Remove the child with the given id from the tree.
    ///
    /// Returns the removed item, or None if no child with the given id exists.
    ///
    /// Calling this will silently remove any recursive grandchildren of this item.
    #[must_use]
    pub fn remove_child(&mut self, child_id: impl Into<NodeId>) -> Option<T> {
        let child_id = child_id.into();
        let i = self
            .children
            .iter()
            .position(|child| child.id == child_id)?;

        fn remove_children<I>(
            node: &TreeNode<I>,
            parents_map: &mut HashMap<NodeId, Option<NodeId>>,
        ) {
            for child in &node.children {
                remove_children(child, parents_map);
            }
            parents_map.remove(&node.id);
        }

        let child = self.children.remove(i);
        remove_children(&child, self.parents_map.parents_map);

        Some(child.item)
    }

    /// Returns a shared handle equivalent to this one.
    pub fn reborrow(&self) -> ArenaRefChildren<'_, T> {
        ArenaRefChildren {
            id: self.id,
            children: &*self.children,
            parents_map: self.parents_map.reborrow(),
        }
    }

    /// Returns a mutable handle equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutChildren<'_, T> {
        ArenaMutChildren {
            id: self.id,
            children: &mut *self.children,
            parents_map: self.parents_map.reborrow_mut(),
        }
    }

    /// Find an arena item among descendants (this node not included).
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        self.reborrow().find(id)
    }

    /// Find an arena item among descendants (this node not included).
    ///
    /// Returns a mutable reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        self.find_mut_inner(id.into())
    }

    fn find_mut_inner(self, id: NodeId) -> Option<ArenaMut<'arena, T>> {
        let parent_id = self.parents_map.parents_map.get(&id)?;

        let id_path = if let Some(parent_id) = parent_id {
            self.parents_map.get_id_path(*parent_id, self.id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children: &'arena mut _ = &mut *self.children;
        while let Some((ancestor_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &mut node_children
                .iter_mut()
                .find(|child| child.id == *ancestor_id)?
                .children;
        }

        let node = node_children.iter_mut().find(|child| child.id == id)?;
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
    #[expect(
        clippy::missing_panics_doc,
        reason = "All ids in the tree should have a parent in the parent map"
    )]
    pub fn get_id_path(self, id: NodeId, start_id: Option<NodeId>) -> Vec<NodeId> {
        let mut path = Vec::new();

        if !self.parents_map.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(current) = current_id {
            path.push(current);
            current_id = *self.parents_map.get(&current).unwrap();
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
