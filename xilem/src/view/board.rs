// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget::{self, SvgElement, WidgetMut};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};

use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};

pub use masonry::widget::BoardParams;

mod kurbo_shape;
mod style_modifier;

pub use kurbo_shape::{AnyGraphicsView, GraphicsView};
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

pub struct BoardElement {
    element: Pod<Box<dyn SvgElement>>,
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

impl<State, Action, V> From<PositionedView<V, State, Action>> for AnyBoardChild<State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action, ViewState: 'static>,
{
    fn from(value: PositionedView<V, State, Action>) -> Self {
        AnyBoardChild::View(positioned(value.view.boxed(), value.params))
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

/// A widget-type-erased positioned child [`View`], can be used within a [`Board`] [`View`]
pub enum AnyBoardChild<State, Action = ()> {
    View(PositionedView<Box<AnyWidgetView<State, Action>>, State, Action>),
    Graphics(Box<AnyGraphicsView<State, Action>>),
}

impl<State, Action, V> PositionedView<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    /// Turns this [`BoardItem`] into an [`AnyBoardChild`]
    pub fn into_any_board(self) -> AnyBoardChild<State, Action> {
        AnyBoardChild::View(positioned(Box::new(self.view), self.params))
    }
}

#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct AnyBoardChildState<State: 'static, Action: 'static> {
    #[allow(clippy::type_complexity)]
    inner: <PositionedView<Box<AnyWidgetView<State, Action>>, State, Action> as View<
        State,
        Action,
        ViewCtx,
    >>::ViewState,

    /// The generational id handling is essentially very similar to that of the `Option<impl ViewSequence>`,
    /// where `None` would represent a Spacer, and `Some` a view
    generation: u64,
}

impl<State, Action> ViewMarker for AnyBoardChild<State, Action> {}
impl<State, Action> View<State, Action, ViewCtx> for AnyBoardChild<State, Action>
where
    State: 'static,
    Action: 'static,
{
    type Element = BoardElement;

    type ViewState = AnyBoardChildState<State, Action>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, view_state) = match self {
            AnyBoardChild::View(view_item) => {
                ctx.with_id(ViewId::new(generation), |ctx| view_item.build(ctx))
            }
            AnyBoardChild::Graphics(shape_item) => {
                let (element, state) =
                    ctx.with_id(ViewId::new(generation), |ctx| shape_item.build(ctx));
                (
                    BoardElement {
                        element: element.inner.svg_boxed().into(),
                    },
                    state,
                )
            }
        };
        (
            element,
            AnyBoardChildState {
                inner: view_state,
                generation,
            },
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        match (prev, self) {
            (AnyBoardChild::View(prev), AnyBoardChild::View(this)) => ctx
                .with_id(ViewId::new(view_state.generation), |ctx| {
                    this.rebuild(prev, &mut view_state.inner, ctx, element)
                }),
            (AnyBoardChild::Graphics(prev), AnyBoardChild::Graphics(this)) => {
                {
                    let mut child = element.parent.child_mut(element.idx);
                    ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                        this.rebuild(prev, &mut view_state.inner, ctx, child.downcast())
                    });
                }
                element
            }
            (AnyBoardChild::View(prev_view), AnyBoardChild::Graphics(new_shape)) => {
                // Run teardown with the old path
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    prev_view.teardown(
                        &mut view_state.inner,
                        ctx,
                        BoardElementMut {
                            parent: element.parent.reborrow_mut(),
                            idx: element.idx,
                        },
                    );
                });
                element.parent.remove_child(element.idx);
                view_state.generation = view_state.generation.wrapping_add(1);
                let (child, child_state) = ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    new_shape.build(ctx)
                });
                view_state.inner = child_state;
                element
                    .parent
                    .insert_child(element.idx, child.inner.svg_boxed());
                element
            }
            (AnyBoardChild::Graphics(prev_shape), AnyBoardChild::View(new_view)) => {
                // Run teardown with the old path
                {
                    let mut child = element.parent.child_mut(element.idx);
                    ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                        prev_shape.teardown(&mut view_state.inner, ctx, child.downcast());
                    });
                }
                element.parent.remove_child(element.idx);
                view_state.generation = view_state.generation.wrapping_add(1);
                let (view_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_view.build(ctx)
                    });
                view_state.inner = child_state;
                element
                    .parent
                    .insert_child(element.idx, view_element.element.inner);
                element
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        match self {
            AnyBoardChild::View(view_item) => {
                view_item.teardown(&mut view_state.inner, ctx, element);
            }
            AnyBoardChild::Graphics(shape_item) => {
                let mut child = element.parent.child_mut(element.idx);
                shape_item.teardown(&mut view_state.inner, ctx, child.downcast());
            }
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for AnyBoardChild");
        if start.routing_id() != view_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        match self {
            AnyBoardChild::View(view_item) => {
                view_item.message(&mut view_state.inner, rest, message, app_state)
            }
            AnyBoardChild::Graphics(shape_item) => {
                shape_item.message(&mut view_state.inner, rest, message, app_state)
            }
        }
    }
}
