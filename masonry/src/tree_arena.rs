// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// TODO - Factor out into a separate crate.
#![allow(dead_code)]

struct TreeNode<Item> {
    id: u64,
    item: Item,
    children: Vec<TreeNode<Item>>,
}

#[derive(Default)]
pub struct TreeArena<Item> {
    roots: Vec<TreeNode<Item>>,
}

pub struct TreeArenaToken<'a, Item> {
    children: &'a Vec<TreeNode<Item>>,
}

impl<'a, Item> Clone for TreeArenaToken<'a, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Item> Copy for TreeArenaToken<'a, Item> {}

pub struct TreeArenaTokenMut<'a, Item> {
    children: &'a mut Vec<TreeNode<Item>>,
}

#[cfg(FALSE)]
struct TreeArena<Item> {
    items: HashMap<u64, UnsafeCell<Item>>,
    parents: HashMap<u64, u64>,
}

#[cfg(FALSE)]
struct TreeArenaToken<'a, Item> {
    arena: &'a TreeArena<Item>,
    id: u64,
}

#[cfg(FALSE)]
struct TreeArenaTokenMut<'a, Item> {
    arena: &'a TreeArena<Item>,
    id: u64,
}

impl<Item> TreeArena<Item> {
    pub fn new() -> Self {
        TreeArena { roots: Vec::new() }
    }

    pub fn root_token(&self) -> TreeArenaToken<'_, Item> {
        TreeArenaToken {
            children: &self.roots,
        }
    }

    pub fn root_token_mut(&mut self) -> TreeArenaTokenMut<'_, Item> {
        TreeArenaTokenMut {
            children: &mut self.roots,
        }
    }

    // TODO - Use list of parents
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
    pub fn has_child(self, id: u64) -> bool {
        for child in self.children {
            if child.id == id {
                return true;
            }
        }
        false
    }

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

    pub fn iter_children(&self) -> impl Iterator<Item = (&'_ Item, TreeArenaToken<'_, Item>)> {
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

    pub fn iter_children(&self) -> impl Iterator<Item = (&'_ Item, TreeArenaToken<'_, Item>)> {
        self.children.iter().map(|child| {
            (
                &child.item,
                TreeArenaToken {
                    children: &child.children,
                },
            )
        })
    }

    pub fn iter_children_mut(
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
    // TODO - Add #[must_use]
    pub fn insert_child(&mut self, child_id: u64, value: Item) -> u64 {
        self.children.push(TreeNode {
            id: child_id,
            item: value,
            children: Vec::new(),
        });
        child_id
    }

    // TODO - How to handle when a subtree is removed?
    #[must_use]
    pub fn remove_child(&mut self, child_id: u64) -> Option<Item> {
        let i = self
            .children
            .iter()
            .position(|child| child.id == child_id)?;
        Some(self.children.remove(i).item)
    }

    pub fn reborrow(&mut self) -> TreeArenaToken<'_, Item> {
        TreeArenaToken {
            children: &*self.children,
        }
    }

    pub fn reborrow_mut(&mut self) -> TreeArenaTokenMut<'_, Item> {
        TreeArenaTokenMut {
            children: &mut *self.children,
        }
    }
}
