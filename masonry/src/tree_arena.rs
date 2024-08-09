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

// TODO:
// - Replace (&T, TreeArenaToken<T>) with "ArenaRef<T>"
// - Replace (&mut T, TreeArenaTokenMut<T>) with "ArenaRefMut<T>"
// - Replace TreeArenaToken<T> with ArenaRefChildren<T>
// - Replace TreeArenaTokenMut<T> with ArenaRefChildrenMut<T>

struct TreeNode<Item> {
    id: u64,
    item: Item,
    children: Vec<TreeNode<Item>>,
}

// TODO - Keep track of parent relationships, and use them to implement
// "find" methods in O(depth) time instead of O(N) time.

/// A container type for a tree of items.
///
/// This type is used to store zero, one or many tree of a given item types. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
#[derive(Default)]
pub struct TreeArena<Item> {
    roots: Vec<TreeNode<Item>>,
}

pub struct ArenaRef<'a, Item> {
    pub parent_id: Option<u64>,
    pub id: Option<u64>,
    pub item: &'a Item,
    pub children: ArenaRefChildren<'a, Item>,
}

pub struct ArenaMut<'a, Item> {
    pub parent_id: Option<u64>,
    pub id: Option<u64>,
    pub item: &'a mut Item,
    pub children: ArenaMutChildren<'a, Item>,
}

/// A reference type giving shared access to an item's children.
///
/// When you borrow an item from a [`TreeArena`], you get two values, returned
/// separately for lifetime reasons: a reference to the item itself, and a token
/// to access its children.
pub struct ArenaRefChildren<'a, Item> {
    id: Option<u64>,
    parent_id: Option<u64>,
    children: &'a Vec<TreeNode<Item>>,
}

/// A reference type giving mutable access to an item's children.
///
/// When you borrow an item from a [`TreeArena`], you get two values, returned
/// separately for lifetime reasons: a reference to the item itself, and a token
/// to access its children.
pub struct ArenaMutChildren<'a, Item> {
    id: Option<u64>,
    parent_id: Option<u64>,
    children: &'a mut Vec<TreeNode<Item>>,
}

// -- MARK: IMPLS ---

impl<'a, Item> Clone for ArenaRefChildren<'a, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Item> Copy for ArenaRefChildren<'a, Item> {}

impl<Item> TreeArena<Item> {
    /// Create an empty tree.
    pub fn new() -> Self {
        TreeArena { roots: Vec::new() }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    pub fn root_token(&self) -> ArenaRefChildren<'_, Item> {
        ArenaRefChildren {
            id: None,
            parent_id: None,
            children: &self.roots,
        }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert_child`](TreeArenaTokenMut::insert_child) on this token
    /// will add a new root to the tree.
    pub fn root_token_mut(&mut self) -> ArenaMutChildren<'_, Item> {
        ArenaMutChildren {
            id: None,
            parent_id: None,
            children: &mut self.roots,
        }
    }

    // TODO - Move into TreeArenaToken::find method
    /// Find an item in the tree.
    ///
    /// Returns a tuple of a shared reference to the item, and a token to access
    /// its children.
    ///
    /// ## Complexity
    ///
    /// O(N) where N is the size of the tree. In future versions, will be O(depth)
    /// or O(1).
    pub fn find(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        fn find_child<Item>(
            node: &TreeNode<Item>,
            parent_id: Option<u64>,
            id: u64,
        ) -> Option<ArenaRef<'_, Item>> {
            if node.id == id {
                return Some(ArenaRef {
                    parent_id,
                    id: Some(node.id),
                    item: &node.item,
                    children: ArenaRefChildren {
                        id: Some(node.id),
                        parent_id,
                        children: &node.children,
                    },
                });
            }
            for child in &node.children {
                if let Some(arena_ref) = find_child(child, Some(node.id), id) {
                    return Some(arena_ref);
                }
            }
            None
        }

        for child in &self.roots {
            if let Some(arena_ref) = find_child(child, None, id) {
                return Some(arena_ref);
            }
        }

        None
    }

    /// Find an item in the tree.
    ///
    /// Returns a tuple of a mutable reference to the item, and a token to access
    /// its children.
    ///
    /// ## Complexity
    ///
    /// O(N) where N is the size of the tree. In future versions, will be O(depth)
    /// or O(1).
    pub fn find_mut(&mut self, id: u64) -> Option<ArenaMut<'_, Item>> {
        fn find_child_mut<Item>(
            node: &mut TreeNode<Item>,
            parent_id: Option<u64>,
            id: u64,
        ) -> Option<ArenaMut<'_, Item>> {
            if node.id == id {
                return Some(ArenaMut {
                    parent_id,
                    id: Some(node.id),
                    item: &mut node.item,
                    children: ArenaMutChildren {
                        id: Some(node.id),
                        parent_id,
                        children: &mut node.children,
                    },
                });
            }
            for child in &mut node.children {
                if let Some(arena_mut) = find_child_mut(child, Some(node.id), id) {
                    return Some(arena_mut);
                }
            }
            None
        }

        for child in &mut self.roots {
            if let Some(arena_mut) = find_child_mut(child, None, id) {
                return Some(arena_mut);
            }
        }

        None
    }

    pub fn get_id_path(&self, id: u64) -> Vec<u64> {
        let mut path = Vec::new();

        if self.find(id).is_none() {
            return path;
        }

        // FIXME
        let mut current_id = Some(id);
        while let Some(id) = current_id {
            path.push(id);
            current_id = self.find(id).unwrap().parent_id;
        }
        path
    }
}

impl<'a, Item> ArenaRefChildren<'a, Item> {
    /// Returns the id of the parent of the item this token is associated with.
    pub fn parent_id(&self) -> Option<u64> {
        self.parent_id
    }

    /// Returns true if the token has a child with the given id.
    pub fn has_child(self, id: u64) -> bool {
        for child in self.children {
            if child.id == id {
                return true;
            }
        }
        false
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a token to access
    /// its children.
    pub fn get_child(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        for child in self.children {
            if child.id == id {
                return Some(ArenaRef {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &child.item,
                    children: ArenaRefChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &child.children,
                    },
                });
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<ArenaRef<'a, Item>> {
        for child in &self.children[..] {
            if child.id == id {
                return Some(ArenaRef {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &child.item,
                    children: ArenaRefChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &child.children,
                    },
                });
            }
        }
        None
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(&self) -> impl Iterator<Item = ArenaRef<'_, Item>> {
        self.children.iter().map(|child| ArenaRef {
            parent_id: self.id,
            id: Some(child.id),
            item: &child.item,
            children: ArenaRefChildren {
                id: Some(child.id),
                parent_id: self.id,
                children: &child.children,
            },
        })
    }
}

impl<'a, Item> ArenaMutChildren<'a, Item> {
    /// Returns the id of the parent of the item this token is associated with.
    pub fn parent_id(&self) -> Option<u64> {
        self.parent_id
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a token to access
    /// its children.
    pub fn get_child(&self, id: u64) -> Option<ArenaRef<'_, Item>> {
        for child in &*self.children {
            if child.id == id {
                return Some(ArenaRef {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &child.item,
                    children: ArenaRefChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &child.children,
                    },
                });
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a token to access
    /// its children.
    pub fn get_child_mut(&mut self, id: u64) -> Option<ArenaMut<'_, Item>> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some(ArenaMut {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &mut child.item,
                    children: ArenaMutChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &mut child.children,
                    },
                });
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<ArenaMut<'a, Item>> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some(ArenaMut {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &mut child.item,
                    children: ArenaMutChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &mut child.children,
                    },
                });
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child_mut`](Self::get_child_mut), except it consumes
    /// the token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child_mut(self, id: u64) -> Option<ArenaMut<'a, Item>> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some(ArenaMut {
                    parent_id: self.id,
                    id: Some(child.id),
                    item: &mut child.item,
                    children: ArenaMutChildren {
                        id: Some(child.id),
                        parent_id: self.id,
                        children: &mut child.children,
                    },
                });
            }
        }
        None
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(&self) -> impl Iterator<Item = ArenaRef<'_, Item>> {
        self.children.iter().map(|child| ArenaRef {
            parent_id: self.id,
            id: Some(child.id),
            item: &child.item,
            children: ArenaRefChildren {
                id: Some(child.id),
                parent_id: self.id,
                children: &child.children,
            },
        })
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children_mut(&mut self) -> impl Iterator<Item = ArenaMut<'_, Item>> {
        self.children.iter_mut().map(|child| ArenaMut {
            parent_id: self.id,
            id: Some(child.id),
            item: &mut child.item,
            children: ArenaMutChildren {
                id: Some(child.id),
                parent_id: self.id,
                children: &mut child.children,
            },
        })
    }

    // TODO - Remove the child_id argument once creation of Widgets is figured out.
    // Return the id instead.
    // TODO - Add #[must_use]
    /// Insert a child into the tree under the item associated with this token.
    ///
    /// The new child will have the given id.
    pub fn insert_child(&mut self, child_id: u64, value: Item) {
        self.children.push(TreeNode {
            id: child_id,
            item: value,
            children: Vec::new(),
        });
    }

    // TODO - How to handle when a subtree is removed?
    #[must_use]
    /// Remove the child with the given id from the tree.
    ///
    /// Returns the removed item, or None if no child with the given id exists.
    ///
    /// Calling this will silently remove any recursive grandchildren of this item.
    pub fn remove_child(&mut self, child_id: u64) -> Option<Item> {
        let i = self
            .children
            .iter()
            .position(|child| child.id == child_id)?;
        Some(self.children.remove(i).item)
    }

    /// Returns a shared token equivalent to this one.
    pub fn reborrow(&mut self) -> ArenaRefChildren<'_, Item> {
        ArenaRefChildren {
            id: self.id,
            parent_id: self.parent_id,
            children: &*self.children,
        }
    }

    /// Returns a mutable token equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutChildren<'_, Item> {
        ArenaMutChildren {
            id: self.id,
            parent_id: self.parent_id,
            children: &mut *self.children,
        }
    }
}

// This is a sketch of what the unsafe version of this code would look like,
// one with an actual arena.
#[cfg(FALSE)]
mod arena_version {
    struct TreeArena<Item> {
        items: HashMap<u64, UnsafeCell<Item>>,
        parents: HashMap<u64, u64>,
    }

    struct TreeArenaToken<'a, Item> {
        arena: &'a TreeArena<Item>,
        id: u64,
    }

    struct TreeArenaTokenMut<'a, Item> {
        arena: &'a TreeArena<Item>,
        id: u64,
    }
}
