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

struct TreeNode<Item> {
    id: u64,
    item: Item,
    children: Vec<TreeNode<Item>>,
}

// TODO - Keep track of parent relationships, and use them to implement
// "find" methods in O(depth) time instead of O(N) time.

#[derive(Default)]
/// A container type for a tree of items.
///
/// This type is used to store zero, one or many tree of a given item types. It
/// will keep track of parent-child relationships, lets you efficiently find
/// an item anywhere in the tree hierarchy, and give you mutable access to this item
/// and its children.
pub struct TreeArena<Item> {
    roots: Vec<TreeNode<Item>>,
}

/// A reference type giving shared access to an item's children.
///
/// When you borrow an item from a [`TreeArena`], you get two values, returned
/// separately for lifetime reasons: a reference to the item itself, and a token
/// to access its children.
pub struct TreeArenaToken<'a, Item> {
    children: &'a Vec<TreeNode<Item>>,
}

impl<'a, Item> Clone for TreeArenaToken<'a, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Item> Copy for TreeArenaToken<'a, Item> {}

/// A reference type giving mutable access to an item's children.
///
/// When you borrow an item from a [`TreeArena`], you get two values, returned
/// separately for lifetime reasons: a reference to the item itself, and a token
/// to access its children.
pub struct TreeArenaTokenMut<'a, Item> {
    children: &'a mut Vec<TreeNode<Item>>,
}

impl<Item> TreeArena<Item> {
    /// Create an empty tree.
    pub fn new() -> Self {
        TreeArena { roots: Vec::new() }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    pub fn root_token(&self) -> TreeArenaToken<'_, Item> {
        TreeArenaToken {
            children: &self.roots,
        }
    }

    /// Returns a token whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert_child`](TreeArenaTokenMut::insert_child) on this token
    /// will add a new root to the tree.
    pub fn root_token_mut(&mut self) -> TreeArenaTokenMut<'_, Item> {
        TreeArenaTokenMut {
            children: &mut self.roots,
        }
    }

    /// Find an item in the tree.
    ///
    /// Returns a tuple of a shared reference to the item, and a token to access
    /// its children.
    ///
    /// ## Complexity
    ///
    /// O(N) where N is the size of the tree. In future versions, will be O(depth)
    /// or O(1).
    pub fn find(&self, id: u64) -> Option<(&Item, TreeArenaToken<'_, Item>)> {
        fn find_child<Item>(
            node: &TreeNode<Item>,
            id: u64,
        ) -> Option<(&Item, TreeArenaToken<'_, Item>)> {
            if node.id == id {
                return Some((
                    &node.item,
                    TreeArenaToken {
                        children: &node.children,
                    },
                ));
            }
            for child in &node.children {
                if let Some((item, token)) = find_child(child, id) {
                    return Some((item, token));
                }
            }
            None
        }

        for child in &self.roots {
            if let Some((item, token)) = find_child(child, id) {
                return Some((item, token));
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
    pub fn find_mut(&mut self, id: u64) -> Option<(&mut Item, TreeArenaTokenMut<'_, Item>)> {
        fn find_child_mut<Item>(
            node: &mut TreeNode<Item>,
            id: u64,
        ) -> Option<(&mut Item, TreeArenaTokenMut<'_, Item>)> {
            if node.id == id {
                return Some((
                    &mut node.item,
                    TreeArenaTokenMut {
                        children: &mut node.children,
                    },
                ));
            }
            for child in &mut node.children {
                if let Some((item, token)) = find_child_mut(child, id) {
                    return Some((item, token));
                }
            }
            None
        }

        for child in &mut self.roots {
            if let Some((item, token)) = find_child_mut(child, id) {
                return Some((item, token));
            }
        }

        None
    }
}

impl<'a, Item> TreeArenaToken<'a, Item> {
    // TODO - Implement this
    fn parent_id(self) -> Option<u64> {
        unimplemented!()
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
    pub fn get_child(&self, id: u64) -> Option<(&'_ Item, TreeArenaToken<'_, Item>)> {
        for child in self.children {
            if child.id == id {
                return Some((
                    &child.item,
                    TreeArenaToken {
                        children: &child.children,
                    },
                ));
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<(&'a Item, TreeArenaToken<'a, Item>)> {
        for child in &self.children[..] {
            if child.id == id {
                return Some((
                    &child.item,
                    TreeArenaToken {
                        children: &child.children,
                    },
                ));
            }
        }
        None
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(
        &self,
    ) -> impl Iterator<Item = (&'_ Item, TreeArenaToken<'_, Item>)> {
        self.children.iter().map(|child| {
            (
                &child.item,
                TreeArenaToken {
                    children: &child.children,
                },
            )
        })
    }
}

impl<'a, Item> TreeArenaTokenMut<'a, Item> {
    // TODO - Implement this
    fn parent_id(&self) -> Option<u64> {
        unimplemented!()
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a shared reference to the child and a token to access
    /// its children.
    pub fn get_child(&self, id: u64) -> Option<(&'_ Item, TreeArenaToken<'_, Item>)> {
        for child in &*self.children {
            if child.id == id {
                return Some((
                    &child.item,
                    TreeArenaToken {
                        children: &child.children,
                    },
                ));
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a token to access
    /// its children.
    pub fn get_child_mut(
        &mut self,
        id: u64,
    ) -> Option<(&'_ mut Item, TreeArenaTokenMut<'_, Item>)> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some((
                    &mut child.item,
                    TreeArenaTokenMut {
                        children: &mut child.children,
                    },
                ));
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child`](Self::get_child), except it consumes the
    /// token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child(self, id: u64) -> Option<(&'a Item, TreeArenaToken<'a, Item>)> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some((
                    &child.item,
                    TreeArenaToken {
                        children: &child.children,
                    },
                ));
            }
        }
        None
    }

    /// Get the child of the item this token is associated with, which has the given id.
    ///
    /// This is the same as [`get_child_mut`](Self::get_child_mut), except it consumes
    /// the token. This is sometimes necesssary to accommodate the borrow checker.
    pub fn into_child_mut(self, id: u64) -> Option<(&'a mut Item, TreeArenaTokenMut<'a, Item>)> {
        for child in &mut self.children[..] {
            if child.id == id {
                return Some((
                    &mut child.item,
                    TreeArenaTokenMut {
                        children: &mut child.children,
                    },
                ));
            }
        }
        None
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children(
        &self,
    ) -> impl Iterator<Item = (&'_ Item, TreeArenaToken<'_, Item>)> {
        self.children.iter().map(|child| {
            (
                &child.item,
                TreeArenaToken {
                    children: &child.children,
                },
            )
        })
    }

    // TODO - This method could not be implemented with an actual arena design.
    // It's currently used for some sanity-checking of widget code, but will
    // likely be removed.
    pub(crate) fn iter_children_mut(
        &mut self,
    ) -> impl Iterator<Item = (&'_ mut Item, TreeArenaTokenMut<'_, Item>)> {
        self.children.iter_mut().map(|child| {
            (
                &mut child.item,
                TreeArenaTokenMut {
                    children: &mut child.children,
                },
            )
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
    pub fn reborrow(&mut self) -> TreeArenaToken<'_, Item> {
        TreeArenaToken {
            children: &*self.children,
        }
    }

    /// Returns a mutable token equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> TreeArenaTokenMut<'_, Item> {
        TreeArenaTokenMut {
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
