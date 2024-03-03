use crate::{
    id::Id,
    view::ElementsSplice,
    widget::{ChangeFlags, Pod},
};
use xilem_core::VecSplice;

use super::Cx;

/// An ElementsSplice that tracks the widget tree structure
pub struct TreeStructureSplice<'a, 'b> {
    current_child_id: Option<Id>,
    splice: VecSplice<'a, 'b, Pod>,
}

impl<'a, 'b> TreeStructureSplice<'a, 'b> {
    pub fn new(elements: &'a mut Vec<Pod>, scratch: &'b mut Vec<Pod>) -> Self {
        Self {
            splice: VecSplice::new(elements, scratch),
            current_child_id: None,
        }
    }
}

impl<'a, 'b> ElementsSplice for TreeStructureSplice<'a, 'b> {
    fn push(&mut self, element: Pod, cx: &mut Cx) {
        cx.tree_structure
            .append_child(cx.element_id(), element.id());
        self.splice.push(element);
    }

    fn mutate(&mut self, _cx: &mut Cx) -> &mut Pod {
        let pod = self.splice.mutate();
        self.current_child_id = Some(pod.id());
        pod
    }

    fn mark(&mut self, changeflags: ChangeFlags, cx: &mut Cx) -> ChangeFlags {
        if changeflags.contains(ChangeFlags::tree_structure()) {
            let current_id = self.current_child_id.take().unwrap();
            let new_id = self.splice.peek().unwrap().id();
            if current_id != new_id {
                cx.tree_structure
                    .change_child(cx.element_id(), self.splice.len() - 1, new_id);
            }
        }

        self.splice.mark(changeflags, cx)
    }

    fn delete(&mut self, n: usize, cx: &mut Cx) {
        let ix = self.splice.len();
        cx.tree_structure
            .delete_descendants(cx.element_id(), ix..ix + n);
        self.splice.delete(n);
    }

    fn len(&self) -> usize {
        self.splice.len()
    }
}
