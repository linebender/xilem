// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget::{self, KurboShape, SvgElement, WidgetMut};
use xilem_core::{
    AnyElement, AnyView, AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement,
    View, ViewElement, ViewMarker, ViewSequence,
};

use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::widget::BoardParams;

mod kurbo_shape;
mod style_modifier;

pub use kurbo_shape::GraphicsView;
pub use style_modifier::{fill, stroke, transform, Fill, GraphicsExt, Stroke, Transform};

pub fn board<State, Action, Seq: BoardSequence<State, Action>>(
    sequence: Seq,
) -> Board<Seq, State, Action> {
    Board {
        sequence,
        phantom: PhantomData,
    }
}

pub struct Board<Seq, State, Action = ()> {
    sequence: Seq,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> ViewMarker for Board<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Board<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: BoardSequence<State, Action>,
{
    type Element = Pod<widget::Board>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::Board::new();
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for BoardElement { element } in elements.into_inner() {
            widget = widget.with_child_pod(element.inner);
        }
        (Pod::new(widget), seq_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        // TODO: Re-use scratch space?
        let mut splice = BoardSplice::new(element);
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
        let mut splice = BoardSplice::new(element);
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

pub type AnyBoardView<State, Action = ()> =
    dyn AnyView<State, Action, ViewCtx, BoardElement> + Send + Sync;

pub struct BoardElement {
    element: Pod<Box<dyn SvgElement>>,
}

pub struct BoardElementMut<'w> {
    parent: WidgetMut<'w, widget::Board>,
    idx: usize,
}

struct BoardSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::Board>,
    scratch: AppendVec<BoardElement>,
}

impl<'w> BoardSplice<'w> {
    fn new(element: WidgetMut<'w, widget::Board>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ViewElement for BoardElement {
    type Mut<'w> = BoardElementMut<'w>;
}

impl SuperElement<BoardElement> for BoardElement {
    fn upcast(child: BoardElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, BoardElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = BoardElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl AnyElement<BoardElement> for BoardElement {
    fn replace_inner(mut this: Self::Mut<'_>, child: BoardElement) -> Self::Mut<'_> {
        this.parent.remove_child(this.idx);
        this.parent.insert_child(this.idx, child.element.inner);
        this
    }
}

impl SuperElement<Pod<KurboShape>> for BoardElement {
    fn upcast(child: Pod<KurboShape>) -> Self {
        BoardElement {
            element: child.inner.svg_boxed().into(),
        }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<KurboShape>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let mut child = this.parent.child_mut(this.idx);
            f(child.downcast())
        };
        (this, r)
    }
}

impl AnyElement<Pod<KurboShape>> for BoardElement {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<KurboShape>) -> Self::Mut<'_> {
        this.parent.remove_child(this.idx);
        this.parent.insert_child(this.idx, child.inner.svg_boxed());
        this
    }
}

impl ElementSplice<BoardElement> for BoardSplice<'_> {
    fn insert(&mut self, BoardElement { element }: BoardElement) {
        self.element.insert_child(self.idx, element.inner);
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<BoardElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for BoardElement { element } in self.scratch.drain() {
            self.element.insert_child(self.idx, element.inner);
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, BoardElement>) -> R) -> R {
        let child = BoardElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, BoardElement>) -> R) -> R {
        let ret = {
            let child = BoardElementMut {
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

/// An ordered sequence of views for a [`Board`] view.
/// See [`ViewSequence`] for more technical details.
pub trait BoardSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, BoardElement>
{
}

impl<Seq, State, Action> BoardSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, BoardElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a positioned item,
/// or being able to use it interchangeably with a shape.
pub trait BoardExt<State, Action>: WidgetView<State, Action> {
    /// Makes this view absolutely positioned in a `Board`.
    fn positioned(self, params: impl Into<BoardParams>) -> PositionedView<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        positioned(self, params)
    }
}

impl<State, Action, V: WidgetView<State, Action>> BoardExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Board`] [`View`]
pub struct PositionedView<V, State, Action> {
    view: V,
    params: BoardParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Makes this view absolutely positioned in a `Board`.
pub fn positioned<V, State, Action>(
    view: V,
    params: impl Into<BoardParams>,
) -> PositionedView<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    PositionedView {
        view,
        params: params.into(),
        phantom: PhantomData,
    }
}

impl<State, Action, V> From<PositionedView<V, State, Action>> for Box<AnyBoardView<State, Action>>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action, ViewState: 'static>,
{
    fn from(value: PositionedView<V, State, Action>) -> Self {
        Box::new(positioned(value.view, value.params))
    }
}

impl<State, Action, V> PositionedView<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    /// Turns this [`BoardItem`] into an [`AnyBoardChild`]
    pub fn into_any_board(self) -> Box<AnyBoardView<State, Action>> {
        self.into()
    }
}

impl<V, State, Action> ViewMarker for PositionedView<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for PositionedView<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = BoardElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx);
        (
            BoardElement {
                element: pod.inner.positioned(self.params).into(),
            },
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
            // if self.params != prev.params {
            //     element
            //         .parent
            //         .update_child_board_params(element.idx, self.params);
            // }
            let mut child = element.parent.child_mut(element.idx);
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast_positioned());
            if self.params.origin != prev.params.origin {
                child.widget.set_origin(self.params.origin);
                child.ctx.request_layout();
            }
            if self.params.size != prev.params.size {
                child.widget.set_size(self.params.size);
                child.ctx.request_layout();
            }
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = element.parent.child_mut(element.idx);
        self.view
            .teardown(view_state, ctx, child.downcast_positioned());
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
