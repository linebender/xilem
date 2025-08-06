// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(unsafe_code, reason = "Purpose is unsafe abstraction")]
use std::cell::UnsafeCell;

use hashbrown::HashMap;

use crate::NodeId;

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
    /// The parent of each node, or `None` if it is the root
    parents: HashMap<NodeId, Option<NodeId>>,
}

/// A reference type giving shared access to an arena item and its children.
///
/// When you borrow an item from a [`TreeArena`], it returns an [`ArenaRef`].
/// You can access its children to get access to child [`ArenaRef`] handles.
#[derive(Debug)]
pub struct ArenaRef<'arena, T> {
    /// Parent of the Node
    pub parent_id: Option<NodeId>,
    /// Item in the node
    pub item: &'arena T,
    /// Children of the node
    pub children: ArenaRefList<'arena, T>,
}

/// A handle giving shared access to a set of arena items.
///
/// See [`ArenaRef`] for more information.
#[derive(Debug)]
pub struct ArenaRefList<'arena, T> {
    /// The associated data arena
    parent_arena: &'arena DataMap<T>,
    /// The parent id for the items
    parent_id: Option<NodeId>,
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
    /// Parent of the Node
    pub parent_id: Option<NodeId>,
    /// Item in the node
    pub item: &'arena mut T,
    /// Children of the node
    pub children: ArenaMutList<'arena, T>,
}

/// A handle giving mutable access to a set of arena items.
///
/// See [`ArenaMut`] for more information.
#[derive(Debug)]
pub struct ArenaMutList<'arena, T> {
    /// The associated data arena
    parent_arena: &'arena mut DataMap<T>,
    /// The parent id for these items
    parent_id: Option<NodeId>,
    /// Array of items
    child_arr: &'arena mut Vec<NodeId>,
}

/// A shared reference to the parent map
#[derive(Clone, Copy, Debug)]
pub struct ArenaMapRef<'arena> {
    parents_map: &'arena HashMap<NodeId, Option<NodeId>>,
}

/// A mutable reference to the parent map
#[derive(Debug)]
pub struct ArenaMapMut<'arena> {
    parents_map: &'arena mut HashMap<NodeId, Option<NodeId>>,
}

// ---

impl<Item> Clone for ArenaRef<'_, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Item> Copy for ArenaRef<'_, Item> {}

impl<T> Clone for ArenaRefList<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Item> Copy for ArenaRefList<'_, Item> {}

impl<T> DataMap<T> {
    fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    fn find_node(&self, id: NodeId) -> Option<&TreeNode<T>> {
        let node_cell = self.items.get(&id)?;

        // SAFETY
        // We need there to be no mutable access to the node
        // Mutable access to the node would imply there is some &mut self
        // As we are taking &self, there can be no mutable access to the node
        // Thus this is safe

        Some(unsafe { node_cell.get().as_ref()? })
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// Time Complexity O(1)
    fn find_inner(&self, parents_map: ArenaMapRef<'_>, id: NodeId) -> Option<ArenaRef<'_, T>> {
        let parent_id = *parents_map.parents_map.get(&id)?;

        let TreeNode { item, .. } = self.find_node(id)?;

        let children = ArenaRefList {
            parent_arena: self,
            parent_id: Some(id),
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
    fn find_mut_inner(
        &mut self,
        parents_map: ArenaMapRef<'_>,
        id: NodeId,
    ) -> Option<ArenaMut<'_, T>> {
        let parent_id = *parents_map.parents_map.get(&id)?;
        let node_cell = self.items.get(&id)?;

        // SAFETY
        //
        // When using this on [`ArenaMutList`] associated with some node,
        // must ensure that `id` is a descendant of that node, otherwise can
        // obtain two mutable references to the same node
        //
        // Similarly we cannot take any other actions that would affect this node,
        // such as removing it or removing a parent (and thus this node) or violate
        // exclusivity by creating a shared reference to the node
        let TreeNode { item, children } = unsafe { node_cell.get().as_mut()? };

        let children = ArenaMutList {
            parent_arena: self,
            parent_id: Some(id),
            child_arr: children,
        };

        Some(ArenaMut {
            parent_id,
            item,
            children,
        })
    }
}

impl<T> TreeArena<T> {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            data_map: DataMap::new(),
            parents: HashMap::new(),
            roots: Vec::new(),
        }
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    pub fn roots(&self) -> (ArenaRefList<'_, T>, ArenaMapRef<'_>) {
        (
            ArenaRefList {
                parent_arena: &self.data_map,
                parent_id: None,
            },
            ArenaMapRef {
                parents_map: &self.parents,
            },
        )
    }

    /// An iterator visiting all root ids in arbitrary order.
    pub fn root_ids(&self) -> impl Iterator<Item = NodeId> {
        self.roots.iter().copied()
    }

    /// Returns a handle whose children are the roots, if any, of the tree.
    ///
    /// Using [`insert`](ArenaMutList::insert) on this handle
    /// will add a new root to the tree.
    pub fn roots_mut(&mut self) -> (ArenaMutList<'_, T>, ArenaMapMut<'_>) {
        // safe as the roots are derived from the arena itself (same as safety for find for non root nodes)
        let roots = &mut self.roots;
        (
            ArenaMutList {
                parent_arena: &mut self.data_map,
                parent_id: None,
                child_arr: roots,
            },
            ArenaMapMut {
                parents_map: &mut self.parents,
            },
        )
    }

    /// Find an item in the tree.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(1).
    pub fn find(&self, id: impl Into<NodeId>) -> (Option<ArenaRef<'_, T>>, ArenaMapRef<'_>) {
        let (roots, map) = self.roots();
        (roots.find(map, id.into()), map)
    }

    /// Find an item in the tree.
    ///
    /// Returns a mutable reference to the item if present.
    pub fn find_mut(
        &mut self,
        id: impl Into<NodeId>,
    ) -> (Option<ArenaMut<'_, T>>, ArenaMapMut<'_>) {
        // safe as derived from the arena itself and has assoc lifetime with the arena
        let (roots, map) = self.roots_mut();
        (roots.find_mut(map.reborrow(), id.into()), map)
    }

    /// Construct the path of items from the given item to the root of the tree.
    ///
    /// The path is in order from the bottom to the top, starting at the given item and ending at
    /// the root.
    ///
    /// If the id is not in the tree, returns an empty vector.
    pub fn get_id_path(&self, id: impl Into<NodeId>) -> Vec<NodeId> {
        let parents_map = ArenaMapRef {
            parents_map: &self.parents,
        };
        parents_map.get_id_path(id.into(), None)
    }

    /// Moves the given child (along with all its children) to the new parent.
    ///
    /// # Panics
    ///
    /// Panics if the parent is actually a child of the to-be-reparented node, or
    /// if the to-be-reparented node is a root node, or
    /// if either node id cannot be found, or
    /// if both given ids are equal.
    pub fn reparent(&mut self, child: impl Into<NodeId>, new_parent: impl Into<NodeId>) {
        let child_id = child.into();
        let new_parent_id = new_parent.into();

        assert_ne!(
            child_id, new_parent_id,
            "expected child to be different from new_parent but both have id #{child_id}"
        );
        assert!(
            !self.get_id_path(new_parent_id).contains(&child_id),
            "cannot reparent because new_parent #{new_parent_id} is a child of the to-be-reparented node #{child_id}"
        );
        assert!(
            !self.roots.contains(&child_id),
            "reparenting of root nodes is currently not supported"
        );

        // ensure new parent id exists
        assert!(
            self.data_map.items.contains_key(&new_parent_id),
            "no node found for new_parent id #{new_parent_id}"
        );

        let old_parent_id = self
            .parents
            .get(&child_id)
            .unwrap_or_else(|| panic!("no node found for child id #{child_id}"))
            .unwrap();

        let arena_map = ArenaMapRef {
            parents_map: &self.parents,
        };

        // Remove child from old parent's children.
        self.data_map
            .find_mut_inner(arena_map, old_parent_id)
            .unwrap()
            .children
            .child_arr
            .retain(|i| *i != child_id);

        // Add child to new parent's children.
        // Safe because we checked that the new parent exists and is not a child of the to-be-reparented node
        self.data_map
            .find_mut_inner(arena_map, new_parent_id)
            .unwrap_or_else(|| panic!("no node found for child id #{new_parent_id}"))
            .children
            .child_arr
            .push(child_id);

        // Update parent reference.
        self.parents.insert(child_id, Some(new_parent_id));
    }
}

impl<T> Default for TreeArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ArenaRef<'_, T> {
    /// Id of the item this handle is associated with.
    pub fn id(&self) -> NodeId {
        self.children
            .parent_id
            .expect("ArenaRefList always has a parent_id when it's a member of ArenaRef")
    }

    /// An iterator visiting all child ids in arbitrary order.
    // NOTE: We're implementing child_ids for ArenaRef instead of ArenaRefList
    // because in an ArenaRefList the implementation for the root ids would be quite inefficient
    // (since the roots are stored in the TreeArena to which the ArenaRefList doesn't have access).
    pub fn child_ids(&self) -> impl IntoIterator<Item = NodeId> {
        self.children
            .parent_arena
            .find_node(self.id())
            .into_iter()
            .flat_map(|c: &TreeNode<T>| &c.children)
            .copied()
    }
}

impl<'arena, T> ArenaRefList<'arena, T> {
    /// Check if id is a descendant of self
    /// O(depth) and the limiting factor for find methods
    /// not from the root
    fn is_descendant(&self, parents_map: ArenaMapRef<'arena>, id: NodeId) -> bool {
        if !self.parent_arena.items.contains_key(&id) {
            // if the id is not in the tree, it is not a descendant
            return false;
        }

        // The arena is derived from the root, and the id is in the tree
        if self.parent_id.is_none() {
            return true;
        }

        // iff the path is empty, there is no path from id to self
        !parents_map.get_id_path(id, self.parent_id).is_empty()
    }

    /// Returns `true` if the list has an element with the given id.
    pub fn has(&self, parents_map: ArenaMapRef<'arena>, id: impl Into<NodeId>) -> bool {
        let child_id = id.into();
        let parent_id = self.parent_id;
        parents_map
            .parents_map
            .get(&child_id)
            .map(|parent| *parent == parent_id) // check if the parent of child is the same as the parent of the arena
            .unwrap_or_default()
    }

    /// Get a shared handle to the element of the list with the given id.
    ///
    /// Return a new [`ArenaRef`]
    pub fn item(
        &self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            self.parent_arena.find_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Get a shared handle to the element of the list with the given id.
    ///
    /// This is the same as [`item`](Self::item), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item(
        self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            self.parent_arena.find_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find(
        self,
        parents_map: ArenaMapRef<'_>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'arena, T>> {
        // the id to search for
        let id: NodeId = id.into();

        if self.is_descendant(parents_map, id) {
            self.parent_arena.find_inner(parents_map, id)
        } else {
            None
        }
    }
}

impl<T> ArenaMut<'_, T> {
    /// Id of the item this handle is associated with
    pub fn id(&self) -> NodeId {
        self.children
            .parent_id
            .expect("ArenaMutList always has a parent_id when it's a member of ArenaMut")
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

impl<'arena, T> ArenaMutList<'arena, T> {
    /// Check if id is a descendant of self
    /// O(depth) and the limiting factor for find methods
    /// not from the root
    fn is_descendant(&self, parents_map: ArenaMapRef<'arena>, id: NodeId) -> bool {
        self.reborrow().is_descendant(parents_map, id)
    }

    /// Returns `true` if the list has an element with the given id.
    pub fn has(&self, parents_map: ArenaMapRef<'arena>, id: impl Into<NodeId>) -> bool {
        self.reborrow().has(parents_map, id)
    }

    /// Get a shared handle to the element of the list with the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a handle to access
    /// its children.
    pub fn item(
        &self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'_, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            self.parent_arena.find_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Get a mutable handle to the element of the list with the given id.
    ///
    /// Returns a tuple of a mutable reference to the child and a handle to access
    /// its children.
    pub fn item_mut(
        &mut self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaMut<'_, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            // safe as we check the node is a direct child node
            self.parent_arena.find_mut_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Get a shared handle to the element of the list with the given id.
    ///
    /// This is the same as [`item`](Self::item), except it consumes the
    /// handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item(
        self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'arena, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            self.parent_arena.find_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Get a mutable handle to the element of the list with the given id.
    ///
    /// This is the same as [`item_mut`](Self::item_mut), except it consumes
    /// the handle. This is sometimes necessary to accommodate the borrow checker.
    pub fn into_item_mut(
        self,
        parents_map: ArenaMapRef<'arena>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        if self.has(parents_map, id) {
            // safe as we check the node is a direct child node
            self.parent_arena.find_mut_inner(parents_map, id)
        } else {
            None
        }
    }

    // TODO - Remove the child_id argument once creation of widgets is figured out.
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
    pub fn insert(
        &mut self,
        parents_map: ArenaMapMut<'_>,
        child_id: impl Into<NodeId>,
        value: T,
    ) -> ArenaMut<'_, T> {
        let child_id: NodeId = child_id.into();
        assert!(
            !parents_map.parents_map.contains_key(&child_id),
            "Key already present"
        );

        parents_map.parents_map.insert(child_id, self.parent_id);

        self.child_arr.push(child_id);

        let node = TreeNode {
            item: value,
            children: Vec::new(),
        };

        self.parent_arena
            .items
            .insert(child_id, Box::new(UnsafeCell::new(node)));

        self.parent_arena
            .find_mut_inner(parents_map.reborrow(), child_id)
            .unwrap()
    }

    // TODO - How to handle when a subtree is removed?
    // Move children to the root?
    // Should this be must use?
    /// Remove the item with the given id from the arena.
    ///
    /// If the id isn't in the list (even if it's e.g. a descendant), does nothing
    /// and returns `None`.
    ///
    /// Else, returns the removed item.
    ///
    /// This will also silently remove any recursive grandchildren of this item.
    #[must_use]
    pub fn remove(
        &mut self,
        parents_map: ArenaMapMut<'_>,
        child_id: impl Into<NodeId>,
    ) -> Option<T> {
        let child_id: NodeId = child_id.into();

        if !self.has(parents_map.reborrow(), child_id) {
            return None;
        }

        fn remove_children<T>(
            id: NodeId,
            mut parents_map: ArenaMapMut<'_>,
            data_map: &mut DataMap<T>,
        ) -> T {
            let node = data_map.items.remove(&id).unwrap().into_inner();
            for child_id in node.children.into_iter() {
                remove_children(child_id, parents_map.reborrow_mut(), data_map);
            }
            parents_map.parents_map.remove(&id);
            node.item
        }
        self.child_arr.retain(|i| *i != child_id);
        Some(remove_children(child_id, parents_map, self.parent_arena))
    }

    /// Returns a shared handle equivalent to this one.
    pub fn reborrow(&self) -> ArenaRefList<'_, T> {
        ArenaRefList {
            parent_arena: self.parent_arena,
            parent_id: self.parent_id,
        }
    }

    /// Returns a mutable handle equivalent to this one.
    ///
    /// This is sometimes useful to work with the borrow checker.
    pub fn reborrow_mut(&mut self) -> ArenaMutList<'_, T> {
        ArenaMutList {
            parent_arena: self.parent_arena,
            parent_id: self.parent_id,
            child_arr: self.child_arr,
        }
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find(
        &self,
        parents_map: ArenaMapRef<'_>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaRef<'_, T>> {
        self.reborrow().find(parents_map, id)
    }

    /// Find an arena item among the list's items and their descendants.
    ///
    /// Returns a shared reference to the item if present.
    ///
    /// ## Complexity
    ///
    /// O(Depth). except access from root which is O(1).
    pub fn find_mut(
        self,
        parents_map: ArenaMapRef<'_>,
        id: impl Into<NodeId>,
    ) -> Option<ArenaMut<'arena, T>> {
        let id = id.into();
        if self.is_descendant(parents_map, id) {
            // safe as we check the node is a descendant
            self.parent_arena.find_mut_inner(parents_map, id)
        } else {
            None
        }
    }

    /// Used in tests to simulate a call to `Self::insert` or `Self::remove` that
    /// triggers a realloc.
    ///
    /// This is an unstable API which can only be used in tests of the `tree_arena` crate itself,
    /// and may change in any release.
    /// It is used to surface potential Use-After-Free (UAF) errors in the code.
    #[doc(hidden)]
    pub fn realloc_inner_storage(&mut self) {
        // By doubling the required capacity (plus a small constant for small capacities),
        // we hopefully guarantee that a reallocation will happen no matter the original capabity.
        let capacity = self.parent_arena.items.capacity();
        let capacity = std::hint::black_box(capacity);
        self.parent_arena.items.reserve(capacity + 32);

        // We try to discard the extra memory.
        // We use black_box to hide the fact that the above call to reserve could be elided.
        if std::hint::black_box(true) {
            self.parent_arena.items.shrink_to_fit();
        }
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
            return Vec::new();
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

        // We've gone all the way to the root without finding start_id.
        if current_id != start_id {
            return Vec::new();
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
