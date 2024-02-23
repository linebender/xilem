use crate::{
    id::Id,
    view::ElementsSplice,
    widget::{ChangeFlags, Pod},
};
use xilem_core::VecSplice;

use super::Cx;

#[derive(Clone, Copy, Debug)]
pub enum SpliceMutation {
    Add(Id),
    Change(Id),
    Delete(usize),
    Skip(usize),
}

/// An ElementsSplice that monitors the tree structure by maintaining a mutation log
pub struct TreeStructureTrackingSplice<'a, 'b, 'c> {
    current_child_id: Option<Id>,
    splice: VecSplice<'a, 'b, Pod>,
    mutations: &'c mut Vec<SpliceMutation>,
}

impl<'a, 'b, 'c> TreeStructureTrackingSplice<'a, 'b, 'c> {
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

impl<'a, 'b, 'c> ElementsSplice for TreeStructureTrackingSplice<'a, 'b, 'c> {
    fn push(&mut self, element: Pod, cx: &mut Cx) {
        self.mutations.push(SpliceMutation::Add(element.id()));
        cx.apply_children_tree_structure_mutations(element.id());
        self.splice.push(element);
    }

    fn mutate(&mut self, _cx: &mut Cx) -> &mut Pod {
        let pod = self.splice.mutate();
        self.current_child_id = Some(pod.id());
        pod
    }

    fn mark(&mut self, changeflags: ChangeFlags, cx: &mut Cx) -> ChangeFlags {
        let mut skip = || {
            if let Some(SpliceMutation::Skip(count)) = self.mutations.last_mut() {
                *count += 1;
            } else {
                self.mutations.push(SpliceMutation::Skip(1));
            }
        };
        // TODO(#160) fine-grained tracking (check whether only this child and not its descendents have changed)
        if !changeflags.is_empty() {
            let current_id = self.current_child_id.take().unwrap();
            // apply children mutations accumulated from the old children
            cx.apply_children_tree_structure_mutations(current_id);
            let new_id = self.splice.peek().unwrap().id();
            if current_id != new_id {
                self.mutations.push(SpliceMutation::Change(new_id));
            } else {
                skip();
            }
        } else {
            skip();
        }

        self.splice.mark(changeflags, cx)
    }

    fn delete(&mut self, n: usize, _cx: &mut Cx) {
        if let Some(SpliceMutation::Delete(count)) = self.mutations.last_mut() {
            *count += n;
        } else {
            self.mutations.push(SpliceMutation::Delete(n));
        }
        self.splice.delete(n);
    }

    fn len(&self, _cx: &mut Cx) -> usize {
        self.splice.len()
    }
}
