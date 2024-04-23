use crate::view::Cx;
use crate::{view::ElementsSplice, widget::ChangeFlags};

use masonry::widget::WidgetMut;
use masonry::{Widget, WidgetId};
use xilem_core::VecSplice;

/// An ElementsSplice that tracks the widget tree structure
pub struct TreeStructureSplice<'a> {
    current_child_id: Option<WidgetId>,
    splice: &'a mut dyn ElementsSplice,
}

impl<'a> TreeStructureSplice<'a> {
    pub fn new(splice: &'a mut dyn ElementsSplice) -> Self {
        Self {
            current_child_id: None,
            splice,
        }
    }
}

impl<'a> ElementsSplice for TreeStructureSplice<'a> {
    fn push(&mut self, element: Box<dyn Widget>, cx: &mut Cx) {
        cx.tree_structure
            .append_child(cx.element_id(), element.id());
        self.splice.push(element, cx);
    }

    fn mutate(&mut self, cx: &mut Cx) -> &mut WidgetMut<'_, Box<dyn Widget>> {
        let pod = self.splice.mutate(cx);
        self.current_child_id = Some(pod.id());
        pod
    }

    fn mark(&mut self, changeflags: ChangeFlags, cx: &mut Cx) -> ChangeFlags {
        if changeflags.contains(ChangeFlags::tree_structure()) {
            let current_id = self.current_child_id.take().unwrap();
            let new_id = self.splice.last_mutated(cx).unwrap().id();
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
            .delete_children(cx.element_id(), ix..ix + n);
        self.splice.delete(n, cx);
    }

    fn len(&self) -> usize {
        self.splice.len()
    }
}
