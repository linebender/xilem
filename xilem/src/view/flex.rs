// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, Axis, CrossAxisAlignment, MainAxisAlignment, WidgetMut},
    Widget, WidgetPod,
};

use crate::{ElementSplice, MasonryView, VecSplice, ViewSequence};

// TODO: Allow configuring flex properties. I think this actually needs its own view trait?
pub fn flex<VT, Marker>(sequence: VT) -> Flex<VT, Marker> {
    Flex {
        phantom: PhantomData,
        sequence,
        axis: Axis::Vertical,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        fill_major_axis: false,
    }
}

pub struct Flex<VT, Marker> {
    sequence: VT,
    axis: Axis,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    phantom: PhantomData<fn() -> Marker>,
}

impl<VT, Marker> Flex<VT, Marker> {
    pub fn direction(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn cross_axis_alignment(mut self, axis: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = axis;
        self
    }

    pub fn main_axis_alignment(mut self, axis: MainAxisAlignment) -> Self {
        self.main_axis_alignment = axis;
        self
    }

    pub fn must_fill_major_axis(mut self, fill_major_axis: bool) -> Self {
        self.fill_major_axis = fill_major_axis;
        self
    }
}

impl<State, Action, Marker: 'static, Seq> MasonryView<State, Action> for Flex<Seq, Marker>
where
    Seq: ViewSequence<State, Action, Marker>,
{
    type Element = widget::Flex;
    type ViewState = Seq::SeqState;

    fn build(
        &self,
        cx: &mut crate::ViewCx,
    ) -> (masonry::WidgetPod<Self::Element>, Self::ViewState) {
        let mut elements = Vec::new();
        let mut scratch = Vec::new();
        let mut splice = VecSplice::new(&mut elements, &mut scratch);
        let seq_state = self.sequence.build(cx, &mut splice);
        let mut view = widget::Flex::for_axis(self.axis)
            .cross_axis_alignment(self.cross_axis_alignment)
            .must_fill_main_axis(self.fill_major_axis)
            .main_axis_alignment(self.main_axis_alignment);
        debug_assert!(
            scratch.is_empty(),
            // TODO: Not at all confident about this, but linear_layout makes this assumption
            "ViewSequence shouldn't leave splice in strange state"
        );
        for item in elements.drain(..) {
            view = view.with_child_pod(item).with_default_spacer();
        }
        (WidgetPod::new(view), seq_state)
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.sequence
            .message(view_state, id_path, message, app_state)
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut crate::ViewCx,
        prev: &Self,
        mut element: widget::WidgetMut<Self::Element>,
    ) {
        if prev.axis != self.axis {
            element.set_direction(self.axis);
            cx.mark_changed();
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            element.set_cross_axis_alignment(self.cross_axis_alignment);
            cx.mark_changed();
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            element.set_main_axis_alignment(self.main_axis_alignment);
            cx.mark_changed();
        }
        if prev.fill_major_axis != self.fill_major_axis {
            element.set_must_fill_main_axis(self.fill_major_axis);
            cx.mark_changed();
        }
        let mut splice = FlexSplice { ix: 0, element };
        self.sequence
            .rebuild(view_state, cx, &prev.sequence, &mut splice);
    }
}

struct FlexSplice<'w> {
    ix: usize,
    element: WidgetMut<'w, widget::Flex>,
}

impl ElementSplice for FlexSplice<'_> {
    fn push(&mut self, element: WidgetPod<Box<dyn masonry::Widget>>) {
        self.element.insert_child_pod(self.ix, element);
        self.element.insert_default_spacer(self.ix);
        self.ix += 2;
    }

    fn mutate(&mut self) -> WidgetMut<Box<dyn Widget>> {
        #[cfg(debug_assertions)]
        let mut iterations = 0;
        #[cfg(debug_assertions)]
        let max = self.element.widget.len();
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
        self.ix / 2
    }
}
