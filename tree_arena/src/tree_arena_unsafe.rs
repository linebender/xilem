// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(unsafe_code, reason = "Purpose is unsafe abstraction")]
use super::NodeId;

use std::cell::UnsafeCell;

use hashbrown::HashMap;

#[derive(Debug)]
struct TreeNode<T> {
    item: T,
    children: Vec<NodeId>,
}

/// Mapping of data for the Tree Arena
#[derive(Debug)]
struct DataMap<T> {
    /// The items in the tree
    items: HashMap<NodeId, Box<UnsafeCell<TreeNode<T>>>>,
    /// The parent of each node, or None if it is the root
    parents: HashMap<NodeId, Option<NodeId>>,
}

/// A container type for a tree of items.
///
/// This type is used to store zero, one or many trees of a given item type. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
#[derive(Debug)]
pub struct TreeArena<T> {
    /// The items in the tree
    data_map: DataMap<T>,
    /// The roots of the tree
    roots: Vec<NodeId>,
}

/// A reference type giving shared access to an arena item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an [`ArenaRef`].
/// You can access it children to get access to child [`ArenaRef`] handles.
#[derive(Debug)]
pub struct ArenaRef<'arena, T> {
    /// Parent of the Node
    pub parent_id: Option<NodeId>,
    /// Item in the node
    pub item: &'arena T,
    /// Children of the node
    pub children: ArenaRefChildren<'arena, T>,
}

/// A handle giving shared access to an arena item's children.
///
/// See [`ArenaRef`] for more information.
#[derive(Debug)]
pub struct ArenaRefChildren<'arena, T> {
    /// The associated data arena
    parent_arena: &'arena DataMap<T>,
    /// The parent id for these children
    id: Option<NodeId>,
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
    /// Parent of the Node
    pub parent_id: Option<NodeId>,
    /// Item in the node
    pub item: &'arena mut T,
    /// Children of the node
    pub children: ArenaMutChildren<'arena, T>,
}

/// A handle giving mutable access to an arena item's children.
///
/// See [`ArenaMut`] for more information.
///
/// This stores all the permissions for what nodes can be accessed from the current node
/// As such if a [`std::mem::swap`] is used to swap the children of two trees,
/// each tree will still have the correct permissions. This also stores the roots, and so
/// that will also be in a consistent state
#[derive(Debug)]
pub struct ArenaMutChildren<'arena, T> {
    /// The associated data arena
    parent_arena: &'arena mut DataMap<T>,
    /// The parent id for these children
    id: Option<NodeId>,
    /// Array of children
    child_arr: &'arena mut Vec<NodeId>,
}

impl<Item> Clone for ArenaRef<'_, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Item> Copy for ArenaRef<'_, Item> {}

impl<T> Clone for ArenaRefChildren<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Item> Copy for ArenaRefChildren<'_, Item> {}

impl<T> DataMap<T> {
    fn new() -> Self {
        Self {
            items: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// Time Complexity O(1)
    fn find_inner(&self, id: NodeId) -> Option<ArenaRef<'_, T>> {
        let parent_id = *self.parents.get(&id)?;
        let node_cell = self.items.get(&id)?;

        // SAFETY
        // We need there to be no mutable access to the node
        // Mutable access to the node would imply there is some &mut self
        // As we are taking &self, there can be no mutable access to the node
        // Thus this is safe

        let TreeNode { item, .. } = unsafe { node_cell.get().as_ref()? };

        let children = ArenaRefChildren {
            parent_arena: self,
            id: Some(id),
        };

        Some(ArenaRef {
            parent_id,
            item,
            children,
        })
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item if present.
    ///
    /// Time Complexity O(1)
    fn find_mut_inner(&mut self, id: NodeId) -> Option<ArenaMut<'_, T>> {
        let parent_id = *self.parents.get(&id)?;
        let node_cell = self.items.get(&id)?;

        // SAFETY
        //
        // When using this on [`ArenaMutChildren`] associated with some node,
        // must ensure that `id` is a descendant of that node, otherwise can
        // obtain two mutable references to the same node
        //
        // Similarly we cannot take any other actions that would affect this node,
        // such as removing it or removing a parent (and thus this node) or violate
        // exclusivity by creating a shared reference to the node
        let TreeNode { item, children } = unsafe { node_cell.get().as_mut()? };

        let children = ArenaMutChildren {
            parent_arena: self,
            id: Some(id),
            child_arr: children,
        };

        Some(ArenaMut {
            parent_id,
            item,
            children,
        })
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If `start_id` is Some, the path ends just before that id instead; `start_id` is not included.
    ///
    /// If there is no path from `start_id` to id, returns the empty vector.
    fn get_id_path(&self, id: NodeId, start_id: Option<NodeId>) -> Vec<NodeId> {
        let mut path = Vec::new();

        if !self.parents.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(current) = current_id {
            path.push(current);
            current_id = *self.parents.get(&current).unwrap();
            if current_id == start_id {
                break;
            }
        }

        // current_id was the last parent node
        // as such if current id is not start_id
        // we have gone to the root and we empty the vec
        if current_id != start_id {
            path.clear();
        }
        path
    }
}

impl<T> TreeArena<T> {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            data_map: DataMap::new(),
            roots: Vec::new(),
        }
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    pub fn root_token(&self) -> ArenaRefChildren<'_, T> {
        ArenaRefChildren {
            parent_arena: &self.data_map,
            id: None,
        }
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert_child`](ArenaMutChildren::insert_child) on this handle
    /// will add a new root to the tree.
    pub fn root_token_mut(&mut self) -> ArenaMutChildren<'_, T> {
        // safe as the roots are derived from the arena itself (same as safety for find for non root nodes)
        let roots = &mut self.roots;
        ArenaMutChildren {
            parent_arena: &mut self.data_map,
            id: None,
            child_arr: roots,
        }
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(1).
    pub fn find(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        self.data_map.find_inner(id.into())
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item if present.
    pub fn find_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        // safe as derived from the arena itself and has assoc lifetime with the arena
        self.data_map.find_mut_inner(id.into())
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If the id is not in the tree, returns an empty vector.
    pub fn get_id_path(&self, id: impl Into<NodeId>) -> Vec<NodeId> {
        self.data_map.get_id_path(id.into(), None)
    }
}

impl<T> Default for TreeArena<T> {
    fn default() -> Self {
        Self::new()
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

impl<'arena, T> ArenaRefChildren<'arena, T> {
    /// Check if id is a descendant of self
    /// O(depth) and the limiting factor for find methods
    /// not from the root
    fn is_descendant(&self, id: NodeId) -> bool {
        if self.parent_arena.items.contains_key(&id) {
            // the id of the parent
            let parent_id = self.id;

            // The arena is derived from the root, and the id is in the tree
            if parent_id.is_none() {
                return true;
            }

            // iff the path is empty, there is no path from id to self
            !self.parent_arena.get_id_path(id, parent_id).is_empty()
        } else {
            // if the id is not in the tree, it is not a descendant
            false
        }
    }

    /// Returns true if there is a child with the given id
    pub fn has_child(&self, id: impl Into<NodeId>) -> bool {
        let child_id = id.into();
        let parent_id = self.id;
        self.parent_arena
            .parents
            .get(&child_id)
            .map(|parent| *parent == parent_id) // check if the parent of child is the same as the parent of the arena
            .unwrap_or_default()
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Return a new [`ArenaRef`]
    pub fn get_child(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        if self.has_child(id) {
            self.parent_arena.find_inner(id)
        } else {
            None
        }
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        if self.has_child(id) {
            self.parent_arena.find_inner(id)
        } else {
            None
        }
    }

    /// Find an arena item among descendants (this node not included).
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        // the id to search for
        let id: NodeId = id.into();

        if self.is_descendant(id) {
            self.parent_arena.find_inner(id)
        } else {
            None
        }
    }
}

impl<T> ArenaMut<'_, T> {
    /// Id of the item this handle is associated with
    #[expect(
        clippy::missing_panics_doc,
        reason = "ArenaMutChildren always has an id when it's a member of ArenaMut"
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

impl<'arena, T> ArenaMutChildren<'arena, T> {
    /// Check if id is a descendant of self
    /// O(depth) and the limiting factor for find methods
    /// not from the root
    fn is_descendant(&self, id: NodeId) -> bool {
        self.reborrow().is_descendant(id)
    }

    /// returns true if there is a child with the given id
    pub fn has_child(&self, id: impl Into<NodeId>) -> bool {
        let child_id = id.into();
        let parent_id = self.id;
        self.parent_arena
            .parents
            .get(&child_id)
            .map(|parent| *parent == parent_id) // check if the parent of child is the same as the parent of the arena
            .unwrap_or_default()
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a handle to access
    /// its children.
    pub fn get_child(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        if self.has_child(id) {
            self.parent_arena.find_inner(id)
        } else {
            None
        }
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a handle to access
    /// its children.
    pub fn get_child_mut(&mut self, id: impl Into<NodeId>) -> Option<ArenaMut<'_, T>> {
        let id = id.into();
        if self.has_child(id) {
            // safe as we check the node is a direct child node
            self.parent_arena.find_mut_inner(id)
        } else {
            None
        }
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: impl Into<NodeId>) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        if self.has_child(id) {
            self.parent_arena.find_inner(id)
        } else {
            None
        }
    }

    /// Get the child of the item this handle is associated with, which has the given id.
    ///
    /// This is the same as [`get_child_mut`](Self::get_child_mut), except it consumes
    /// the handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        if self.has_child(id) {
            // safe as we check the node is a direct child node
            self.parent_arena.find_mut_inner(id)
        } else {
            None
        }
    }

    // TODO - Remove the child_id argument once creation of Widgets is figured out.
    // Return the id instead.
    /// Insert a child into the tree under the item associated with this handle.
    ///
    /// The new child will have the given id.
    ///
    /// # Panics
    ///
    /// The `insert_child` method will panic if the arena already contains a child
    /// with the given id.
    pub fn insert_child(&mut self, child_id: impl Into<NodeId>, value: T) {
        let child_id: NodeId = child_id.into();
        assert!(
            !self.parent_arena.parents.contains_key(&child_id),
            "Key already present"
        );

        self.parent_arena.parents.insert(child_id, self.id);

        self.child_arr.push(child_id);

        let node = TreeNode {
            item: value,
            children: Vec::new(),
        };

        self.parent_arena
            .items
            .insert(child_id, Box::new(UnsafeCell::new(node)));
    }

    // TODO - How to handle when a subtree is removed?
    // Move children to the root?
    // Should this be must use?
    /// Remove the child with the given id from the tree.
    ///
    /// Returns the removed item, or None if no child with the given id exists.
    ///
    /// Calling this will silently remove any recursive grandchildren of this item.
    #[must_use]
    pub fn remove_child(&mut self, child_id: impl Into<NodeId>) -> Option<T> {
        let child_id: NodeId = child_id.into();
        if self.has_child(child_id) {
            fn remove_children<T>(id: NodeId, data_map: &mut DataMap<T>) -> T {
                let node = data_map.items.remove(&id).unwrap().into_inner();
                for child_id in node.children.into_iter() {
                    remove_children(child_id, data_map);
                }
                data_map.parents.remove(&id);
                node.item
            }
            self.child_arr.retain(|i| *i != child_id);
            Some(remove_children(child_id, self.parent_arena))
        } else {
            None
        }
    }

    /// Returns a shared handle equivalent to this one.
    pub fn reborrow(&self) -> ArenaRefChildren<'_, T> {
        ArenaRefChildren {
            parent_arena: self.parent_arena,
            id: self.id,
        }
    }

    /// Returns a mutable handle equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutChildren<'_, T> {
        ArenaMutChildren {
            parent_arena: self.parent_arena,
            id: self.id,
            child_arr: self.child_arr,
        }
    }

    /// Find an arena item among descendants (this node not included).
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find(&self, id: impl Into<NodeId>) -> Option<ArenaRef<'_, T>> {
        self.reborrow().find(id)
    }

    /// Find an arena item among descendants (this node not included).
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find_mut(self, id: impl Into<NodeId>) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        if self.is_descendant(id) {
            // safe as we check the node is a descendant
            self.parent_arena.find_mut_inner(id)
        } else {
            None
        }
    }
}
