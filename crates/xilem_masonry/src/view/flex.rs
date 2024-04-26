use std::marker::PhantomData;

use masonry::{
    widget::{self, WidgetMut},
    Widget, WidgetPod,
};

use crate::{ElementSplice, MasonryView, VecSplice, ViewId, ViewSequence};

// TODO: Allow configuring flex properties. I think this actually needs its own view trait?
pub fn flex<VT, Marker>(sequence: VT) -> Flex<VT, Marker> {
    Flex {
        phantom: PhantomData,
        sequence,
    }
}

pub struct Flex<VT, Marker> {
    sequence: VT,
    phantom: PhantomData<fn() -> Marker>,
}

impl<State, Action, Marker: 'static, Seq> MasonryView<State, Action> for Flex<Seq, Marker>
where
    Seq: ViewSequence<State, Action, Marker>,
{
    type State = Seq::State;
    type Element = widget::Flex;

    fn build(
        &self,
        cx: &mut crate::ViewCx,
    ) -> (ViewId, Self::State, masonry::WidgetPod<Self::Element>) {
        let mut elements = Vec::new();
        let mut scratch = Vec::new();
        let mut splice = VecSplice::new(&mut elements, &mut scratch);
        let seq_state = self.sequence.build(cx, &mut splice);
        let mut view = widget::Flex::column();
        debug_assert!(
            scratch.is_empty(),
            // TODO: Not at all confident about this, but linear_layout makes this assumption
            "ViewSequence shouldn't leave splice in strange state"
        );
        for item in elements.drain(..) {
            view = view.with_child_pod(item).with_default_spacer();
        }
        (ViewId::next_with_type::<Self>(), seq_state, WidgetPod::new(view))
    }

    fn message(
        &self,
        id_path: &[crate::ViewId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.sequence.message(id_path, state, message, app_state)
    }

    fn rebuild(
        &self,
        cx: &mut crate::ViewCx,
        prev: &Self,
        _id: &mut ViewId,
        state: &mut Self::State,
        element: widget::WidgetMut<Self::Element>,
    ) -> crate::ChangeFlags {
        let mut splice = FlexSplice { ix: 0, element };
        self.sequence
            .rebuild(cx, &prev.sequence, state, &mut splice)
    }
}

struct FlexSplice<'w> {
    ix: usize,
    element: WidgetMut<'w, widget::Flex>,
}

impl ElementSplice for FlexSplice<'_> {
    fn push(&mut self, element: WidgetPod<Box<dyn masonry::Widget>>) {
        self.element.insert_child_pod(self.ix, element);
        self.ix += 1;
    }

    fn mutate(&mut self) -> WidgetMut<Box<dyn Widget>> {
        #[cfg(debug_assertions)]
        let mut iterations = 0;
        #[cfg(debug_assertions)]
        let max = self.element.len();
        loop {
            #[cfg(debug_assertions)]
            {
                if iterations > max {
                    panic!("Got into infinite loop in FlexSplice::mutate");
                }
                iterations += 1;
            }
            let child = self.element.child_mut(self.ix);
            if child.is_some() {
                break;
            }
            self.ix += 1;
        }
        let child = self.element.child_mut(self.ix).unwrap();
        self.ix += 1;
        child
    }

    fn delete(&mut self, n: usize) {
        let mut deleted_count = 0;
        while deleted_count < n {
            {
                // TODO: use a drain/retain type method
                let element = self.element.child_mut(self.ix);
                if element.is_some() {
                    deleted_count += 1;
                }
            }
            self.element.remove_child(self.ix);
        }
    }

    fn len(&self) -> usize {
        self.ix
    }
}
