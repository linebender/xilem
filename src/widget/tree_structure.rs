use crate::id::Id;
use std::{collections::HashMap, ops::RangeBounds};

/// The pure structure (parent/children relations via ids) of the widget tree.
#[derive(Debug, Default, Clone)]
pub struct TreeStructure {
    parent: HashMap<Id, Id>,
    children: HashMap<Id, Vec<Id>>,
}

impl TreeStructure {
    pub fn parent(&self, id: Id) -> Option<Id> {
        self.parent.get(&id).copied()
    }

    pub fn children(&self, id: Id) -> Option<&[Id]> {
        self.children.get(&id).map(Vec::as_slice)
    }

    pub fn is_descendant_of(&self, mut id: Id, ancestor: Id) -> bool {
        while let Some(parent) = self.parent(id) {
            if parent == ancestor {
                return true;
            }
            id = parent;
        }
        false
    }

    pub(crate) fn append_child(&mut self, parent_id: Id, id: Id) {
        self.parent
            .entry(id)
            .and_modify(|parent| {
                *parent = parent_id;
            })
            .or_insert(parent_id);
        self.children
            .entry(parent_id)
            .and_modify(|children| {
                children.push(id);
            })
            .or_insert_with(|| vec![id]);
    }

    /// # Panics
    ///
    /// When the `parent_id` doesn't exist in the structure or `idx` is out of bounds this will panic
    pub(crate) fn change_child(&mut self, parent_id: Id, idx: usize, new_id: Id) {
        let children = self
            .children
            .get_mut(&parent_id)
            .unwrap_or_else(|| panic!("{parent_id:?} doesn't have any child"));
        let old_id = children[idx];
        children[idx] = new_id;

        self.parent.remove(&old_id);
        self.parent
            .entry(new_id)
            .and_modify(|parent| {
                *parent = parent_id;
            })
            .or_insert(parent_id);
    }

    /// Deletes all children of `parent_id` within the `range` and their descendants (recursively)
    pub(crate) fn delete_descendants(&mut self, parent_id: Id, range: impl RangeBounds<usize>) {
        let Some(children) = self.children.get_mut(&parent_id) else {
            return;
        };
        let mut to_delete: Vec<Id> = children.drain(range).collect();

        while let Some(id) = to_delete.pop() {
            if let Some(child_ids) = self.children.remove(&id) {
                to_delete.extend(child_ids);
            }
            self.parent.remove(&id);
        }
        let Some(children) = self.children.get_mut(&parent_id) else {
            return;
        };
        if children.is_empty() {
            self.children.remove(&parent_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutates_simple_tree_structure() {
        let mut tree_structure = TreeStructure::default();

        let parent = Id::next();
        let child1 = Id::next();
        let child2 = Id::next();
        let child3 = Id::next();

        // append children
        tree_structure.append_child(parent, child1);
        tree_structure.append_child(parent, child2);
        tree_structure.append_child(parent, child3);
        let children = tree_structure.children(parent).unwrap();
        assert_eq!(children, [child1, child2, child3]);
        assert_eq!(tree_structure.parent(child1), Some(parent));
        assert_eq!(tree_structure.parent(child2), Some(parent));
        assert_eq!(tree_structure.parent(child3), Some(parent));

        // change children
        let child2_new = Id::next();
        tree_structure.change_child(parent, 1, child2_new);
        let children = tree_structure.children(parent).unwrap();
        assert_eq!(children, [child1, child2_new, child3]);
        assert_eq!(tree_structure.parent(child1), Some(parent));
        assert_eq!(tree_structure.parent(child2), None);
        assert_eq!(tree_structure.parent(child2_new), Some(parent));
        assert_eq!(tree_structure.parent(child3), Some(parent));

        // delete children
        tree_structure.delete_descendants(parent, 0..2);
        let children = tree_structure.children(parent).unwrap();
        assert_eq!(children, [child3]);
        assert_eq!(tree_structure.parent(child1), None);
        assert_eq!(tree_structure.parent(child2), None);
        assert_eq!(tree_structure.parent(child2_new), None);
        assert_eq!(tree_structure.parent(child3), Some(parent));

        // delete all children
        tree_structure.delete_descendants(parent, ..);
        let children = tree_structure.children(parent);
        assert_eq!(children, None);
        assert_eq!(tree_structure.parent(child3), None);
    }

    #[test]
    fn deletes_descendants() {
        let mut tree_structure = TreeStructure::default();
        let parent = Id::next();
        let child1 = Id::next();
        let child2 = Id::next();
        let child3 = Id::next();
        tree_structure.append_child(parent, child1);
        tree_structure.append_child(parent, child2);
        tree_structure.append_child(parent, child3);

        let child3_child1 = Id::next();
        let child3_child2 = Id::next();
        tree_structure.append_child(child3, child3_child1);
        tree_structure.append_child(child3, child3_child2);
        let child3_child1_child1 = Id::next();
        tree_structure.append_child(child3_child1, child3_child1_child1);
        tree_structure.delete_descendants(parent, 1..);
        let children = tree_structure.children(parent).unwrap();
        assert_eq!(children, [child1]);
        assert_eq!(tree_structure.parent(child1), Some(parent));
        assert_eq!(tree_structure.parent.len(), 1);
        assert_eq!(tree_structure.children.len(), 1);
    }

    #[test]
    fn is_descendant_of() {
        let mut tree_structure = TreeStructure::default();
        let parent = Id::next();
        let child1 = Id::next();
        let child2 = Id::next();
        let child3 = Id::next();
        tree_structure.append_child(parent, child1);
        tree_structure.append_child(parent, child2);
        tree_structure.append_child(parent, child3);

        let child3_child1 = Id::next();
        let child3_child2 = Id::next();
        tree_structure.append_child(child3, child3_child1);
        tree_structure.append_child(child3, child3_child2);
        let child3_child1_child1 = Id::next();
        tree_structure.append_child(child3_child1, child3_child1_child1);
        assert!(tree_structure.is_descendant_of(child3_child1_child1, child3_child1));
        assert!(tree_structure.is_descendant_of(child3_child1_child1, child3));
        assert!(tree_structure.is_descendant_of(child3_child1, parent));
        assert!(!tree_structure.is_descendant_of(child3_child1, child2));
        assert!(!tree_structure.is_descendant_of(parent, child3_child1));
    }
}
