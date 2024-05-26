// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, Axis, CrossAxisAlignment, MainAxisAlignment, WidgetMut},
    Widget, WidgetPod,
};
use xilem_core::{AppendVec, ElementSplice, View, ViewSequence};

use crate::{Pod, ViewCtx};

// TODO: Create a custom ViewSequence dynamic element for this
pub fn flex<Seq, Marker>(sequence: Seq) -> Flex<Seq, Marker> {
    Flex {
        phantom: PhantomData,
        sequence,
        axis: Axis::Vertical,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        fill_major_axis: false,
    }
}

pub struct Flex<Seq, Marker> {
    sequence: Seq,
    axis: Axis,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    phantom: PhantomData<fn() -> Marker>,
}

impl<Seq, Marker> Flex<Seq, Marker> {
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

impl<State, Action, Seq, Marker: 'static> View<State, Action, ViewCtx> for Flex<Seq, Marker>
where
    Seq: ViewSequence<State, Action, ViewCtx, Pod<Box<dyn Widget>>, Marker>,
{
    type Element = Pod<widget::Flex>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::Flex::for_axis(self.axis)
            .cross_axis_alignment(self.cross_axis_alignment)
            .with_default_spacer()
            .must_fill_main_axis(self.fill_major_axis)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for item in elements.into_inner() {
            widget = widget.with_child_pod(item.inner).with_default_spacer();
        }
        (WidgetPod::new(widget).into(), seq_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: WidgetMut<widget::Flex>,
    ) {
        if prev.axis != self.axis {
            element.set_direction(self.axis);
            ctx.mark_changed();
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            element.set_cross_axis_alignment(self.cross_axis_alignment);
            ctx.mark_changed();
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            element.set_main_axis_alignment(self.main_axis_alignment);
            ctx.mark_changed();
        }
        if prev.fill_major_axis != self.fill_major_axis {
            element.set_must_fill_main_axis(self.fill_major_axis);
            ctx.mark_changed();
        }
        // TODO: Re-use scratch space?
        let mut splice = FlexSplice {
            // Skip the initial spacer which is always present
            ix: 1,
            element,
            scratch: AppendVec::default(),
        };
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: <Self::Element as xilem_core::ViewElement>::Mut<'_>,
    ) {
        let mut splice = FlexSplice {
            // Skip the initial spacer which is always present
            ix: 1,
            element,
            scratch: AppendVec::default(),
        };
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

struct FlexSplice<'w> {
    ix: usize,
    element: WidgetMut<'w, widget::Flex>,
    scratch: AppendVec<Pod<Box<dyn Widget>>>,
}

impl ElementSplice<Pod<Box<dyn Widget>>> for FlexSplice<'_> {
    fn push(&mut self, element: Pod<Box<dyn masonry::Widget>>) {
        self.element.insert_child_pod(self.ix, element.inner);
        // Insert a spacer after the child
        self.element.insert_default_spacer(self.ix + 1);
        self.ix += 2;
    }
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<Pod<Box<dyn Widget>>>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            self.element.insert_child_pod(self.ix, element.inner);
            self.element.insert_default_spacer(self.ix + 1);
            self.ix += 2;
        }
        ret
    }

    fn mutate<R>(
        &mut self,
        f: impl FnOnce(<Pod<Box<dyn Widget>> as xilem_core::ViewElement>::Mut<'_>) -> R,
    ) -> R {
        let child = self
            .element
            .child_mut(self.ix)
            .expect("ElementSplice::mutate won't overflow");
        let ret = f(child);
        // Skip past the implicit spacer as well as this child
        self.ix += 2;
        ret
    }

    fn delete<R>(
        &mut self,
        f: impl FnOnce(<Pod<Box<dyn Widget>> as xilem_core::ViewElement>::Mut<'_>) -> R,
    ) -> R {
        let child = self
            .element
            .child_mut(self.ix)
            .expect("ElementSplice::mutate won't overflow");
        let ret = f(child);
        self.element.remove_child(self.ix);
        // Also remove the implicit spacer
        // TODO: Make the spacers be explicit?
        self.element.remove_child(self.ix);

        ret
    }

    fn skip(&mut self, n: usize) {
        self.ix += n * 2;
    }
}
