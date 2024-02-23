use crate::id::Id;
use std::collections::HashMap;

/// The pure structure (parent/children relations via ids) of the widget tree.
#[derive(Debug, Default, Clone)]
pub struct TreeStructure {
    parent: HashMap<Id, Option<Id>>,
    // TODO this is taken from the druid lasagna branch, is the Option here intentional, i.e. to track all parentless ids with None?
    children: HashMap<Option<Id>, Vec<Id>>,
}

impl TreeStructure {
    pub fn parent(&self, id: Id) -> Option<Option<Id>> {
        self.parent.get(&id).copied()
    }

    pub fn children(&self, id: Option<Id>) -> Option<&[Id]> {
        self.children.get(&id).map(Vec::as_slice)
    }

    pub fn is_descendant_of(&self, mut id: Id, ancestor: Id) -> bool {
        while let Some(parent) = self.parent(id).flatten() {
            if parent == ancestor {
                return true;
            }
            id = parent;
        }
        false
    }

    // TODO: Should any of the following methods return a Result or panic?
    pub(crate) fn append_child(&mut self, parent_id: Id, id: Id) {
        let parent_id = Some(parent_id);
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

    pub(crate) fn change_child(&mut self, parent_id: Id, idx: usize, new_id: Id) {
        let parent_id = Some(parent_id);
        let mut old_id = None;

        self.children
            .entry(parent_id)
            .and_modify(|children| {
                // TODO Result instead of panic when out of bounds?
                old_id = Some(children[idx]);
                children[idx] = new_id;
            })
            .or_insert_with(|| vec![new_id]); // TODO this should not happen, Result/unwrap?

        if let Some(old_id) = old_id {
            self.parent.remove(&old_id);
            self.parent
                .entry(new_id)
                .and_modify(|parent| {
                    *parent = parent_id;
                })
                .or_insert(parent_id);
        }
    }

    pub(crate) fn delete_children(&mut self, parent_id: Id, range: std::ops::Range<usize>) {
        let parent_id = Some(parent_id);
        let children = &self.children[&parent_id][range.clone()];
        for child in children {
            self.parent.remove(child);
        }
        self.children.get_mut(&parent_id).unwrap().drain(range);
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
        let children = tree_structure.children(Some(parent)).unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], child1);
        assert_eq!(children[1], child2);
        assert_eq!(children[2], child3);
        assert_eq!(tree_structure.parent(child1), Some(Some(parent)));
        assert_eq!(tree_structure.parent(child2), Some(Some(parent)));
        assert_eq!(tree_structure.parent(child3), Some(Some(parent)));

        // change children
        let child2_new = Id::next();
        tree_structure.change_child(parent, 1, child2_new);
        let children = tree_structure.children(Some(parent)).unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], child1);
        assert_eq!(children[1], child2_new);
        assert_eq!(children[2], child3);
        assert_eq!(tree_structure.parent(child1), Some(Some(parent)));
        assert_eq!(tree_structure.parent(child2), None);
        assert_eq!(tree_structure.parent(child2_new), Some(Some(parent)));
        assert_eq!(tree_structure.parent(child3), Some(Some(parent)));

        // delete children
        tree_structure.delete_children(parent, 0..2);
        let children = tree_structure.children(Some(parent)).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], child3);
        assert_eq!(tree_structure.parent(child1), None);
        assert_eq!(tree_structure.parent(child2), None);
        assert_eq!(tree_structure.parent(child2_new), None);
        assert_eq!(tree_structure.parent(child3), Some(Some(parent)));
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
