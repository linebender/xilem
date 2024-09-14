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

#![allow(dead_code)]

use std::collections::HashMap;

struct TreeNode<Item> {
    id: u64,
    item: Item,
    children: Vec<TreeNode<Item>>,
}

/// A container type for a tree of items.
///
/// This type is used to store zero, one or many tree of a given item types. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
#[derive(Default)]
pub struct TreeArena<Item> {
    roots: Vec<TreeNode<Item>>,
    parents_map: HashMap<u64, Option<u64>>,
}

/// A reference type giving shared access to an item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an `ArenaRef`.
/// You can iterate over the children to get access to child `ArenaRef` handles.
pub struct ArenaRef<'a, Item> {
    pub parent_id: Option<u64>,
    pub item: &'a Item,
    pub children: ArenaRefChildren<'a, Item>,
}

/// A reference type giving mutable access to an item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an `ArenaMut`.
/// This struct holds three fields:
///  - the id of its parent.
///  - a reference to the item itself.
///  - an `ArenaMutChildren` handle to access its children.
///
/// Because the latter two are disjoint references, you can mutate the node's value
/// and its children independently without invalidating the references.
///
/// You can iterate over the children to get access to child `ArenaMut` handles.
pub struct ArenaMut<'a, Item> {
    pub parent_id: Option<u64>,
    pub item: &'a mut Item,
    pub children: ArenaMutChildren<'a, Item>,
}

/// A reference type giving shared access to an item's children.
///
/// See [`ArenaRef`] for more information.
pub struct ArenaRefChildren<'a, Item> {
    id: Option<u64>,
    children: &'a Vec<TreeNode<Item>>,
    parents_map: ArenaMapRef<'a>,
}

/// A reference type giving mutable access to an item's children.
///
/// See [`ArenaMut`] for more information.
pub struct ArenaMutChildren<'a, Item> {
    id: Option<u64>,
    children: &'a mut Vec<TreeNode<Item>>,
    parents_map: ArenaMapMut<'a>,
}

#[derive(Clone, Copy)]
pub struct ArenaMapRef<'a> {
    parents_map: &'a HashMap<u64, Option<u64>>,
}

pub struct ArenaMapMut<'a> {
    parents_map: &'a mut HashMap<u64, Option<u64>>,
}

// -- MARK: IMPLS ---

impl<'a, Item> Clone for ArenaRef<'a, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Item> Copy for ArenaRef<'a, Item> {}

impl<'a, Item> Clone for ArenaRefChildren<'a, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Item> Copy for ArenaRefChildren<'a, Item> {}

impl<Item> TreeArena<Item> {
    /// Create an empty tree.
    pub fn new() -> Self {
        TreeArena {
            roots: Vec::new(),
            parents_map: HashMap::new(),
        }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    pub fn root_token(&self) -> ArenaRefChildren<'_, Item> {
        ArenaRefChildren {
            id: None,
            children: &self.roots,
            parents_map: ArenaMapRef {
                parents_map: &self.parents_map,
            },
        }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert_child`](ArenaMutChildren::insert_child) on this token
    /// will add a new root to the tree.
    pub fn root_token_mut(&mut self) -> ArenaMutChildren<'_, Item> {
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
    /// Returns a shared reference to the item.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        self.root_token().find(id)
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find_mut(&mut self, id: u64) -> Option<ArenaMut<'_, Item>> {
        self.root_token_mut().find_mut(id)
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If the id is not in the tree, returns an empty vector.
    pub fn get_id_path(&self, id: u64) -> Vec<u64> {
        let parents_map = ArenaMapRef {
            parents_map: &self.parents_map,
        };
        parents_map.get_id_path(id, None)
    }
}

impl<Item> TreeNode<Item> {
    fn arena_ref<'a>(
        &'a self,
        parent_id: Option<u64>,
        parents_map: &'a HashMap<u64, Option<u64>>,
    ) -> ArenaRef<'a, Item> {
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

    fn arena_mut<'a>(
        &'a mut self,
        parent_id: Option<u64>,
        parents_map: &'a mut HashMap<u64, Option<u64>>,
    ) -> ArenaMut<'a, Item> {
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

impl<'a, Item> ArenaRef<'a, Item> {
    /// Id of the item this handle is associated with.
    pub fn id(&self) -> u64 {
        // ArenaRefChildren always has an id when it's a member of ArenaRef
        self.children.id.unwrap()
    }
}

impl<'a, Item> ArenaMut<'a, Item> {
    /// Id of the item this handle is associated with.
    pub fn id(&self) -> u64 {
        // ArenaMutChildren always has an id when it's a member of ArenaMut
        self.children.id.unwrap()
    }

    /// Returns a shared token equivalent to this one.
    pub fn reborrow(&mut self) -> ArenaRef<'_, Item> {
        ArenaRef {
            parent_id: self.parent_id,
            item: self.item,
            children: self.children.reborrow(),
        }
    }

    /// Returns a mutable token equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMut<'_, Item> {
        ArenaMut {
            parent_id: self.parent_id,
            item: self.item,
            children: self.children.reborrow_mut(),
        }
    }
}

impl<'a, Item> ArenaRefChildren<'a, Item> {
    /// Returns true if the token has a child with the given id.
    pub fn has_child(self, id: u64) -> bool {
        self.children.iter().any(|child| child.id == id)
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a token to access
    /// its children.
    pub fn get_child(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<ArenaRef<'a, Item>> {
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(&self) -> impl Iterator<Item = ArenaRef<'_, Item>> {
        self.children
            .iter()
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(self, id: u64) -> Option<ArenaRef<'a, Item>> {
        let parent_id = self.parents_map.parents_map.get(&id)?;

        let id_path = if let Some(parent_id) = parent_id {
            self.parents_map.get_id_path(*parent_id, self.id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children = &*self.children;
        while let Some((id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &node_children
                .iter()
                .find(|child| child.id == *id)
                .unwrap()
                .children;
        }

        let node = node_children.iter().find(|child| child.id == id).unwrap();
        Some(node.arena_ref(*parent_id, &*self.parents_map.parents_map))
    }
}

impl<'a, Item> ArenaMutChildren<'a, Item> {
    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a token to access
    /// its children.
    pub fn get_child(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a token to access
    /// its children.
    pub fn get_child_mut(&mut self, id: u64) -> Option<ArenaMut<'_, Item>> {
        self.children
            .iter_mut()
            .find(|child| child.id == id)
            .map(|child| child.arena_mut(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<ArenaRef<'a, Item>> {
        self.children
            .iter()
            .find(|child| child.id == id)
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child_mut`](Self::get_child_mut), except it consumes
    /// the token. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_child_mut(self, id: u64) -> Option<ArenaMut<'a, Item>> {
        self.children
            .iter_mut()
            .find(|child| child.id == id)
            .map(|child| child.arena_mut(self.id, self.parents_map.parents_map))
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(&self) -> impl Iterator<Item = ArenaRef<'_, Item>> {
        self.children
            .iter()
            .map(|child| child.arena_ref(self.id, self.parents_map.parents_map))
    }

    // TODO - Remove the child_id argument once creation of Widgets is figured out.
    // Return the id instead.
    // TODO - Add #[must_use]
    /// Insert a child into the tree under the item associated with this token.
    ///
    /// The new child will have the given id.
    ///
    /// # Panics
    ///
    /// The `insert_child` method will panic if the arena already contains a child
    /// with the given id.
    pub fn insert_child(&mut self, child_id: u64, value: Item) {
        assert!(!self.parents_map.parents_map.contains_key(&child_id));
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
    pub fn remove_child(&mut self, child_id: u64) -> Option<Item> {
        let i = self
            .children
            .iter()
            .position(|child| child.id == child_id)?;

        fn remove_children<I>(node: &TreeNode<I>, parents_map: &mut HashMap<u64, Option<u64>>) {
            parents_map.remove(&node.id);
            for child in &node.children {
                remove_children(child, parents_map);
            }
        }

        let child = self.children.remove(i);
        remove_children(&child, self.parents_map.parents_map);

        Some(child.item)
    }

    /// Returns a shared token equivalent to this one.
    pub fn reborrow(&self) -> ArenaRefChildren<'_, Item> {
        ArenaRefChildren {
            id: self.id,
            children: &*self.children,
            parents_map: self.parents_map.reborrow(),
        }
    }

    /// Returns a mutable token equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutChildren<'_, Item> {
        ArenaMutChildren {
            id: self.id,
            children: &mut *self.children,
            parents_map: self.parents_map.reborrow_mut(),
        }
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        self.reborrow().find(id)
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item.
    ///
    /// ## Complexity
    ///
    /// O(Depth). In future implementations, this will be O(1).
    pub fn find_mut(self, id: u64) -> Option<ArenaMut<'a, Item>> {
        let parent_id = self.parents_map.parents_map.get(&id)?;

        let id_path = if let Some(parent_id) = parent_id {
            self.parents_map.get_id_path(*parent_id, self.id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_children: &'a mut _ = &mut *self.children;
        while let Some((id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_children = &mut node_children
                .iter_mut()
                .find(|child| child.id == *id)
                .unwrap()
                .children;
        }

        let node = node_children
            .iter_mut()
            .find(|child| child.id == id)
            .unwrap();
        Some(node.arena_mut(*parent_id, &mut *self.parents_map.parents_map))
    }
}

impl<'a> ArenaMapRef<'a> {
    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If start_id is Some, the path ends just before that id instead; start_id is not included.
    ///
    /// If there is no path from start_id to id, returns an empty vector.
    pub fn get_id_path(&self, id: u64, start_id: Option<u64>) -> Vec<u64> {
        let mut path = Vec::new();

        if !self.parents_map.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(id) = current_id {
            path.push(id);
            current_id = *self.parents_map.get(&id).unwrap();
        }

        if let Some(start_id) = start_id {
            while let Some(id) = path.pop() {
                if id == start_id {
                    break;
                }
            }
        }

        path
    }
}
impl<'a> ArenaMapMut<'a> {
    /// Returns a shared token equivalent to this one.
    pub fn reborrow(&self) -> ArenaMapRef<'_> {
        ArenaMapRef {
            parents_map: self.parents_map,
        }
    }

    /// Returns a mutable token equivalent to this one.
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
    /// If start_id is Some, the path ends just before that id instead; start_id is not included.
    ///
    /// If there is no path from start_id to id, returns an empty vector.
    pub fn get_id_path(&self, id: u64, start_id: Option<u64>) -> Vec<u64> {
        self.reborrow().get_id_path(id, start_id)
    }
}

// This is a sketch of what the unsafe version of this code would look like,
// one with an actual arena.
#[cfg(FALSE)]
mod arena_version {
    struct TreeArena<Item> {
        items: HashMap<u64, UnsafeCell<Item>>,
        parents: HashMap<u64, Option<u64>>,
    }

    struct ArenaRefChildren<'a, Item> {
        arena: &'a TreeArena<Item>,
        id: u64,
    }

    struct ArenaMutChildren<'a, Item> {
        arena: &'a TreeArena<Item>,
        id: u64,
    }
}
