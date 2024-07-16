// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, Axis, CrossAxisAlignment, MainAxisAlignment, WidgetMut},
    Widget,
};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, Mut, SuperElement, View, ViewElement, ViewSequence,
};

use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::widget::FlexParams;

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
    Seq: ViewSequence<State, Action, ViewCtx, FlexElement, Marker>,
{
    type Element = Pod<widget::Flex>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::Flex::for_axis(self.axis)
            .cross_axis_alignment(self.cross_axis_alignment)
            .must_fill_main_axis(self.fill_major_axis)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = match child {
                FlexElement::Child(child, params) => {
                    widget.with_flex_child_pod(child.inner, params)
                }
                FlexElement::FixedSpacer(size) => widget.with_spacer(size),
                FlexElement::FlexSpacer(flex) => widget.with_flex_spacer(flex),
            }
        }
        (Pod::new(widget), seq_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
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
        let mut splice = FlexSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.is_empty());
        splice.element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = FlexSplice::new(element);
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

pub enum FlexElement {
    // Avoid making the enum massive for the spacer cases by boxing
    Child(Box<Pod<Box<dyn Widget>>>, FlexParams),
    FixedSpacer(f64),
    FlexSpacer(f64),
}

pub struct FlexElementMut<'w> {
    parent: WidgetMut<'w, widget::Flex>,
    idx: usize,
}

struct FlexSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::Flex>,
    scratch: AppendVec<FlexElement>,
}

impl<'w> FlexSplice<'w> {
    fn new(element: WidgetMut<'w, widget::Flex>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ViewElement for FlexElement {
    type Mut<'w> = FlexElementMut<'w>;
}

impl SuperElement<FlexElement> for FlexElement {
    fn upcast(child: FlexElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, FlexElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = FlexElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>> for FlexElement {
    fn upcast(child: Pod<W>) -> Self {
        FlexElement::Child(Box::new(child.inner.boxed().into()), FlexParams::default())
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = this
                .parent
                .child_mut(this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<FlexElement> for FlexSplice<'_> {
    fn insert(&mut self, element: FlexElement) {
        match element {
            FlexElement::Child(child, params) => {
                self.element
                    .insert_flex_child_pod(self.idx, child.inner, params);
            }
            FlexElement::FixedSpacer(len) => self.element.insert_spacer(self.idx, len),
            FlexElement::FlexSpacer(len) => self.element.insert_flex_spacer(self.idx, len),
        };
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<FlexElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            match element {
                FlexElement::Child(child, params) => {
                    self.element
                        .insert_flex_child_pod(self.idx, child.inner, params);
                }
                FlexElement::FixedSpacer(len) => self.element.insert_spacer(self.idx, len),
                FlexElement::FlexSpacer(len) => self.element.insert_flex_spacer(self.idx, len),
            };
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, FlexElement>) -> R) -> R {
        let child = FlexElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, FlexElement>) -> R) -> R {
        let ret = {
            let child = FlexElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        self.element.remove_child(self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }
}

pub struct FlexItem<V, State, Action> {
    view: V,
    params: FlexParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn flex_item<V, State, Action>(
    view: V,
    params: impl Into<FlexParams>,
) -> FlexItem<V, State, Action> {
    FlexItem {
        params: params.into(),
        view,
        phantom: PhantomData,
    }
}

impl<State, Action, V> View<State, Action, ViewCtx> for FlexItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = FlexElement;

    type ViewState = V::ViewState;

    fn build(&self, cx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(cx);
        (
            FlexElement::Child(Box::new(pod.inner.boxed().into()), self.params),
            state,
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            if self.params != prev.params {
                element
                    .parent
                    .update_child_flex_params(element.idx, self.params);
            }
            let mut child = element
                .parent
                .child_mut(element.idx)
                .expect("FlexWrapper always has a widget child");
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast());
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = element
            .parent
            .child_mut(element.idx)
            .expect("FlexWrapper always has a widget child");
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum FlexSpacer {
    Fixed(f64),
    Flex(f64),
}

impl<State, Action> View<State, Action, ViewCtx> for FlexSpacer {
    type Element = FlexElement;

    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let el = match self {
            FlexSpacer::Fixed(len) => FlexElement::FixedSpacer(*len),
            FlexSpacer::Flex(flex) => FlexElement::FlexSpacer(*flex),
        };
        (el, ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if self != prev {
            match self {
                FlexSpacer::Fixed(len) => element.parent.update_spacer_fixed(element.idx, *len),
                FlexSpacer::Flex(flex) => element.parent.update_spacer_flex(element.idx, *flex),
            };
        }
        element
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[xilem_core::ViewId],
        _: DynMessage,
        _: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        unreachable!()
    }
}
