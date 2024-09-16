// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, KurboShape, Shape, WidgetMut},
    Widget,
};
use vello::kurbo::{Affine, Point, Stroke, Vec2};
use vello::peniko::{Brush, Fill};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};

use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};

pub use masonry::widget::{Axis, BoardParams, CrossAxisAlignment, MainAxisAlignment};

pub fn board<State, Action, Seq: BoardSequence<State, Action>>(
    sequence: Seq,
) -> Board<Seq, State, Action> {
    Board {
        sequence,
        origin: Point::ZERO,
        scale: Vec2::new(1., 1.),
        phantom: PhantomData,
    }
}

pub struct Board<Seq, State, Action = ()> {
    sequence: Seq,
    origin: Point,
    scale: Vec2,
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
        for child in elements.into_inner() {
            widget = match child {
                BoardElement::View(pod, params) => widget.with_child_pod(pod.inner, params),
                BoardElement::Shape(shape) => widget.with_shape_child(shape),
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
        if prev.origin != self.origin {
            element.set_origin(self.origin);
            ctx.mark_changed();
        }
        if prev.scale != self.scale {
            element.set_scale(self.scale);
            ctx.mark_changed();
        }
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

#[allow(clippy::large_enum_variant)]
pub enum BoardElement {
    View(Pod<Box<dyn Widget>>, BoardParams),
    Shape(Shape),
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

// impl<W: Widget> SuperElement<Pod<W>> for BoardElement {
//     fn upcast(child: Pod<W>) -> Self {
//         BoardElement {
//             element: child.inner.boxed().into(),
//             params: BoardParams::default(),
//         }
//     }

//     fn with_downcast_val<R>(
//         mut this: Mut<'_, Self>,
//         f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
//     ) -> (Mut<'_, Self>, R) {
//         let ret = {
//             let mut child = this
//                 .parent
//                 .child_mut(this.idx)
//                 .expect("This is supposed to be a widget");
//             let downcast = child.downcast();
//             f(downcast)
//         };

//         (this, ret)
//     }
// }

impl ElementSplice<BoardElement> for BoardSplice<'_> {
    fn insert(&mut self, element: BoardElement) {
        match element {
            BoardElement::View(pod, params) => {
                self.element.insert_child_pod(self.idx, pod.inner, params);
            }
            BoardElement::Shape(shape) => self.element.insert_shape_child(self.idx, shape),
        }
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<BoardElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            match element {
                BoardElement::View(pod, params) => {
                    self.element.insert_child_pod(self.idx, pod.inner, params);
                }
                BoardElement::Shape(shape) => self.element.insert_shape_child(self.idx, shape),
            }
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
    fn positioned(self, params: impl Into<BoardParams>) -> BoardViewItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        board_item(self, params)
    }
}

impl<State, Action, V: WidgetView<State, Action>> BoardExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Board`] [`View`]
pub struct BoardViewItem<V, State, Action> {
    view: V,
    params: BoardParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Makes this view absolutely positioned in a `Board`.
pub fn board_item<V, State, Action>(
    view: V,
    params: impl Into<BoardParams>,
) -> BoardViewItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    BoardViewItem {
        params: params.into(),
        view,
        phantom: PhantomData,
    }
}

impl<State, Action, V> From<BoardViewItem<V, State, Action>> for AnyBoardChild<State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action, ViewState: 'static>,
{
    fn from(value: BoardViewItem<V, State, Action>) -> Self {
        AnyBoardChild::View(board_item(value.view.boxed(), value.params))
    }
}

impl<V, State, Action> ViewMarker for BoardViewItem<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for BoardViewItem<V, State, Action>
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
            BoardElement::View(pod.inner.boxed().into(), self.params),
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
                    .update_child_board_params(element.idx, self.params);
            }
            let mut child = element.parent.child_mut(element.idx);
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
        let mut child = element.parent.child_mut(element.idx);
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

pub struct ShapeItem {
    shape: KurboShape,
    transform: Affine,
    fill_style: Fill,
    fill_brush: Brush,
    fill_brush_transform: Option<Affine>,
    stroke_style: Stroke,
    stroke_brush: Brush,
    stroke_brush_transform: Option<Affine>,
}

impl ShapeItem {
    pub fn transform(mut self, transform: Affine) -> Self {
        self.transform = transform;
        self
    }
    pub fn fill_style(mut self, fill_style: Fill) -> Self {
        self.fill_style = fill_style;
        self
    }
    pub fn fill_brush(mut self, fill_brush: impl Into<Brush>) -> Self {
        self.fill_brush = fill_brush.into();
        self
    }
    pub fn fill_brush_transform(mut self, fill_brush_transform: impl Into<Option<Affine>>) -> Self {
        self.fill_brush_transform = fill_brush_transform.into();
        self
    }
    pub fn stroke_style(mut self, stroke_style: Stroke) -> Self {
        self.stroke_style = stroke_style;
        self
    }
    pub fn stroke_brush(mut self, stroke_brush: impl Into<Brush>) -> Self {
        self.stroke_brush = stroke_brush.into();
        self
    }
    pub fn stroke_brush_transform(
        mut self,
        stroke_brush_transform: impl Into<Option<Affine>>,
    ) -> Self {
        self.stroke_brush_transform = stroke_brush_transform.into();
        self
    }

    pub fn into_any_board<State, Action>(self) -> AnyBoardChild<State, Action> {
        AnyBoardChild::Shape(Box::new(self))
    }
}

pub trait ShapeExt {
    fn view(self) -> ShapeItem;
}

impl<T> ShapeExt for T
where
    KurboShape: From<T>,
{
    fn view(self) -> ShapeItem {
        ShapeItem {
            shape: self.into(),
            transform: Default::default(),
            fill_style: Fill::NonZero,
            fill_brush: Default::default(),
            fill_brush_transform: Default::default(),
            stroke_style: Default::default(),
            stroke_brush: Default::default(),
            stroke_brush_transform: Default::default(),
        }
    }
}

impl<State, Action> From<ShapeItem> for AnyBoardChild<State, Action> {
    fn from(shape: ShapeItem) -> Self {
        AnyBoardChild::Shape(Box::new(shape))
    }
}

impl ViewMarker for ShapeItem {}
// This impl doesn't require a view id, as it neither receives, nor sends any messages
// If this should ever change, it's necessary to adjust the `AnyBoardChild` `View` impl
impl<State, Action> View<State, Action, ViewCtx> for ShapeItem {
    type Element = BoardElement;
    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let Self {
            shape,
            transform,
            fill_style,
            fill_brush,
            fill_brush_transform,
            stroke_style,
            stroke_brush,
            stroke_brush_transform,
        } = self;
        let mut shape = Shape::new(shape.clone());
        shape.set_transform(*transform);
        shape.set_fill_style(*fill_style);
        shape.set_fill_brush(fill_brush.clone());
        shape.set_fill_brush_transform(*fill_brush_transform);
        shape.set_stroke_style(stroke_style.clone());
        shape.set_stroke_brush(stroke_brush.clone());
        shape.set_stroke_brush_transform(*stroke_brush_transform);
        (BoardElement::Shape(shape), ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            let mut child = element.parent.child_mut(element.idx);
            let mut child = child.downcast::<Shape>();
            if self.shape != prev.shape {
                child.set_shape(self.shape.clone());
                ctx.mark_changed();
            }
            if self.transform != prev.transform {
                child.set_transform(self.transform);
                ctx.mark_changed();
            }
            if self.fill_style != prev.fill_style {
                child.set_fill_style(self.fill_style);
                ctx.mark_changed();
            }
            if self.fill_brush != prev.fill_brush {
                child.set_fill_brush(self.fill_brush.clone());
                ctx.mark_changed();
            }
            if self.fill_brush_transform != prev.fill_brush_transform {
                child.set_fill_brush_transform(self.fill_brush_transform);
                ctx.mark_changed();
            }
            if self.stroke_style.width != prev.stroke_style.width
                || self.stroke_style.join != prev.stroke_style.join
                || self.stroke_style.miter_limit != prev.stroke_style.miter_limit
                || self.stroke_style.start_cap != prev.stroke_style.start_cap
                || self.stroke_style.end_cap != prev.stroke_style.end_cap
                || self.stroke_style.dash_pattern != prev.stroke_style.dash_pattern
                || self.stroke_style.dash_offset != prev.stroke_style.dash_offset
            {
                child.set_stroke_style(self.stroke_style.clone());
                ctx.mark_changed();
            }
            if self.stroke_brush != prev.stroke_brush {
                child.set_stroke_brush(self.stroke_brush.clone());
                ctx.mark_changed();
            }
            if self.stroke_brush_transform != prev.stroke_brush_transform {
                child.set_stroke_brush_transform(self.stroke_brush_transform);
                ctx.mark_changed();
            }
        }

        element
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[xilem_core::ViewId],
        msg: DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale(msg)
    }
}

/// A widget-type-erased positioned child [`View`], can be used within a [`Board`] [`View`]
pub enum AnyBoardChild<State, Action = ()> {
    View(BoardViewItem<Box<AnyWidgetView<State, Action>>, State, Action>),
    Shape(Box<ShapeItem>),
}

impl<State, Action, V> BoardViewItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    /// Turns this [`BoardItem`] into an [`AnyBoardChild`]
    pub fn into_any_board(self) -> AnyBoardChild<State, Action> {
        AnyBoardChild::View(board_item(Box::new(self.view), self.params))
    }
}

#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct AnyBoardChildState<State: 'static, Action: 'static> {
    /// Just the optional view state of the positioned item view
    #[allow(clippy::type_complexity)]
    inner: Option<
        <BoardViewItem<Box<AnyWidgetView<State, Action>>, State, Action> as View<
            State,
            Action,
            ViewCtx,
        >>::ViewState,
    >,
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
                let (element, state) =
                    ctx.with_id(ViewId::new(generation), |ctx| view_item.build(ctx));
                (element, Some(state))
            }
            AnyBoardChild::Shape(shape_item) => {
                let (element, _) = ctx.with_id(ViewId::new(generation), |ctx| {
                    View::<(), (), ViewCtx>::build(shape_item, ctx)
                });
                (element, None)
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
                    this.rebuild(prev, view_state.inner.as_mut().unwrap(), ctx, element)
                }),
            (AnyBoardChild::Shape(prev), AnyBoardChild::Shape(this)) => {
                View::<(), (), ViewCtx>::rebuild(this, prev, &mut (), ctx, element)
            }
            (AnyBoardChild::View(prev_view), AnyBoardChild::Shape(new_shape)) => {
                // Run teardown with the old path
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    prev_view.teardown(
                        view_state.inner.as_mut().unwrap(),
                        ctx,
                        BoardElementMut {
                            parent: element.parent.reborrow_mut(),
                            idx: element.idx,
                        },
                    );
                });
                element.parent.remove_child(element.idx);
                view_state.inner = None;
                view_state.generation = view_state.generation.wrapping_add(1);
                let (spacer_element, ()) = ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    View::<(), (), ViewCtx>::build(new_shape, ctx)
                });

                match spacer_element {
                    BoardElement::View(_, _) => unreachable!(),
                    BoardElement::Shape(shape) => {
                        element.parent.insert_shape_child(element.idx, shape);
                    }
                };
                element
            }
            (AnyBoardChild::Shape(prev_shape), AnyBoardChild::View(new_view)) => {
                // Run teardown with the old path
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    View::<(), (), ViewCtx>::teardown(
                        prev_shape,
                        &mut (),
                        ctx,
                        BoardElementMut {
                            parent: element.parent.reborrow_mut(),
                            idx: element.idx,
                        },
                    );
                });
                element.parent.remove_child(element.idx);
                view_state.inner = None;
                view_state.generation = view_state.generation.wrapping_add(1);
                let (view_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_view.build(ctx)
                    });
                view_state.inner = Some(child_state);
                match view_element {
                    BoardElement::View(pod, params) => {
                        element
                            .parent
                            .insert_child_pod(element.idx, pod.inner, params);
                    }
                    BoardElement::Shape(_) => unreachable!(),
                };
                element
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        match self {
            AnyBoardChild::View(view_item) => {
                view_item.teardown(view_state.inner.as_mut().unwrap(), ctx, element);
            }
            AnyBoardChild::Shape(shape_item) => {
                View::<(), (), ViewCtx>::teardown(shape_item, &mut (), ctx, element);
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
                view_item.message(view_state.inner.as_mut().unwrap(), rest, message, app_state)
            }
            AnyBoardChild::Shape(shape_item) => {
                shape_item.message(&mut (), rest, message, app_state)
            }
        }
    }
}
