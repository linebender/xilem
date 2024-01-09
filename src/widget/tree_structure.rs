use std::collections::HashMap;

use xilem_core::VecSplice;

use crate::{
    id::Id,
    widget::{ChangeFlags, Pod},
};

use crate::view::ElementsSplice;

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

    pub fn is_descendant_of(&self, id: Id, ancestor: Id) -> bool {
        let mut id = id;
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

    // TODO: currently if there are no children added/mutated the tree will not be mutated.
    //       Should the parent be inserted if there aren't any children? Should this happen elsewhere?
    pub(crate) fn apply_splice_mutations(&mut self, parent_id: Id, mutations: &[SpliceMutation]) {
        let mut idx = 0;
        for mutation in mutations {
            match mutation {
                SpliceMutation::Add(id) => {
                    self.append_child(parent_id, *id);
                    idx += 1;
                }
                SpliceMutation::Change(new_id) => {
                    self.change_child(parent_id, idx, *new_id);
                    idx += 1;
                }
                SpliceMutation::Delete(n) => {
                    self.delete_children(parent_id, idx..(idx + n));
                }
                SpliceMutation::Skip(n) => {
                    idx += n;
                }
            }
        }
    }
}

pub enum SpliceMutation {
    Add(Id),
    Change(Id),
    Delete(usize),
    Skip(usize),
}

pub struct TreeTrackerSplice<'a, 'b, 'c> {
    current_child_id: Option<Id>,
    splice: VecSplice<'a, 'b, Pod>,
    mutations: &'c mut Vec<SpliceMutation>,
}

impl<'a, 'b, 'c> TreeTrackerSplice<'a, 'b, 'c> {
    pub fn new(
        elements: &'a mut Vec<Pod>,
        scratch: &'b mut Vec<Pod>,
        mutations: &'c mut Vec<SpliceMutation>,
    ) -> Self {
        mutations.clear();
        Self {
            splice: VecSplice::new(elements, scratch),
            current_child_id: None,
            mutations,
        }
    }
}

impl<'a, 'b, 'c> ElementsSplice for TreeTrackerSplice<'a, 'b, 'c> {
    fn push(&mut self, element: Pod) {
        self.mutations.push(SpliceMutation::Add(element.id()));
        self.splice.push(element);
    }

    fn mutate(&mut self) -> &mut Pod {
        let pod = self.splice.mutate();
        self.current_child_id = Some(pod.id());
        pod
    }

    fn mark(&mut self, changeflags: ChangeFlags) -> ChangeFlags {
        let mut skip = || {
            if let Some(SpliceMutation::Skip(count)) = self.mutations.last_mut() {
                *count += 1;
            } else {
                self.mutations.push(SpliceMutation::Skip(1));
            }
        };
        // TODO fine-grained tracking (check whether only this child and not its descendents have changed)
        if !changeflags.is_empty() {
            let old_id = self.current_child_id.take().unwrap();
            let new_id = self.splice.peek().unwrap().id();
            if old_id != new_id {
                self.mutations.push(SpliceMutation::Change(new_id));
            } else {
                skip();
            }
        } else {
            skip();
        }

        self.splice.mark(changeflags)
    }

    fn delete(&mut self, n: usize) {
        if let Some(SpliceMutation::Delete(count)) = self.mutations.last_mut() {
            *count += n;
        } else {
            self.mutations.push(SpliceMutation::Delete(n));
        }
        self.splice.delete(n);
    }

    fn len(&self) -> usize {
        self.splice.len()
    }
}
