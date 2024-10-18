// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, WidgetMut},
    Widget,
};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};

use crate::{Pod, ViewCtx, WidgetView};
use taffy;
use taffy::Style;

pub fn taffy_layout<State, Action, Seq: TaffySequence<State, Action>>(
    sequence: Seq,
    style: Style,
) -> TaffyLayout<Seq, State, Action> {
    TaffyLayout {
        sequence,
        style,
        phantom: PhantomData,
    }
}

pub struct TaffyLayout<Seq, State, Action = ()> {
    sequence: Seq,
    style: Style,
    /// Used to associate the State and Action in the call to `.taffy_layout()` with the State and
    /// Action used in the View implementation, to allow inference to flow backwards, allowing
    /// State and Action to be inferred properly
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> ViewMarker for TaffyLayout<Seq, State, Action> {}

impl<State, Action, Seq> View<State, Action, ViewCtx> for TaffyLayout<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: TaffySequence<State, Action>,
{
    type Element = Pod<widget::TaffyLayout>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::TaffyLayout::new(self.style.clone());
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = match child {
                TaffyElement::Child(child, params) => widget.with_child_pod(child.inner, params),
            }
        }
        (ctx.new_pod(widget), seq_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.style != self.style {
            element.set_style(self.style.clone());
        }

        let mut splice = TaffySplice::new(element);
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
        let mut splice = TaffySplice::new(element);
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

// Used to become a reference form for editing. It's provided to rebuild and teardown.
impl ViewElement for TaffyElement {
    type Mut<'w> = TaffyElementMut<'w>;
}

// Used to allow the item to be used as a generic item in ViewSequence.
impl SuperElement<TaffyElement, ViewCtx> for TaffyElement {
    fn upcast(_ctx: &mut ViewCtx, child: TaffyElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, TaffyElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = TaffyElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for TaffyElement {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        // Getting here means that the widget didn't use .with_taffy_style.
        // Uses the default Taffy style.
        TaffyElement::Child(ctx.boxed_pod(child), Style::default())
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

// Used for building and rebuilding the ViewSequence
impl ElementSplice<TaffyElement> for TaffySplice<'_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<TaffyElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            match element {
                TaffyElement::Child(child, style) => {
                    self.element
                        .insert_taffy_child_pod(self.idx, child.inner, style);
                }
            };
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: TaffyElement) {
        match element {
            TaffyElement::Child(child, params) => {
                self.element
                    .insert_taffy_child_pod(self.idx, child.inner, params);
            }
        };
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, TaffyElement>) -> R) -> R {
        let child = TaffyElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, TaffyElement>) -> R) -> R {
        let ret = {
            let child = TaffyElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        self.element.remove_child(self.idx);
        ret
    }
}

/// `TaffySequence` is what allows an input to Taffy that contains all the elements.
pub trait TaffySequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, TaffyElement>
{
}

impl<Seq, State, Action> TaffySequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, TaffyElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a taffy item
pub trait TaffyExt<State, Action>: WidgetView<State, Action> {
    // TODO: Documentation
    fn with_taffy_style(self, params: impl Into<Style>) -> TaffyItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        taffy_item(self, params)
    }
}

impl<State, Action, V: WidgetView<State, Action>> TaffyExt<State, Action> for V {}

pub enum TaffyElement {
    Child(Pod<Box<dyn Widget>>, Style),
}

pub struct TaffyElementMut<'w> {
    parent: WidgetMut<'w, widget::TaffyLayout>,
    idx: usize,
}

// Used for manipulating the ViewSequence.
pub struct TaffySplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::TaffyLayout>,
    scratch: AppendVec<TaffyElement>,
}

impl<'w> TaffySplice<'w> {
    fn new(element: WidgetMut<'w, widget::TaffyLayout>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

/// A `WidgetView` that can be used within a [`Taffy`] [`View`]
pub struct TaffyItem<V, State, Action> {
    view: V,
    style: Style,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn taffy_item<V, State, Action>(
    view: V,
    style: impl Into<Style>,
) -> TaffyItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    TaffyItem {
        view,
        style: style.into(),
        phantom: PhantomData,
    }
}

impl<V, State, Action> ViewMarker for TaffyItem<V, State, Action> {}

impl<State, Action, V> View<State, Action, ViewCtx> for TaffyItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = TaffyElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx);
        (TaffyElement::Child(ctx.boxed_pod(pod), self.style.clone()), state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            if self.style != prev.style {
                element
                    .parent
                    .update_child_taffy_params(element.idx, self.style.clone());
            }
            let mut child = element
                .parent
                .child_mut(element.idx)
                .expect("TaffyWrapper always has a widget child");
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
            .expect("TaffyWrapper always has a widget child");
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}
