// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{widget::KurboShape, Affine};
use vello::{
    kurbo,
    peniko::{self, Brush},
};
use xilem_core::{AnyView, DynMessage, MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{Pod, ViewCtx};

use super::{AnyBoardChild, GraphicsView};

pub struct Transform<V, State, Action> {
    child: V,
    transform: Affine,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub struct Fill<V, State, Action> {
    child: V,
    mode: peniko::Fill,
    brush: Brush,
    brush_transform: Option<Affine>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub struct Stroke<V, State, Action> {
    child: V,
    style: kurbo::Stroke,
    brush: Brush,
    brush_transform: Option<Affine>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn transform<State, Action, V>(child: V, transform: Affine) -> Transform<V, State, Action> {
    Transform {
        child,
        transform,
        phantom: PhantomData,
    }
}

pub fn fill<State, Action, V>(child: V, brush: impl Into<Brush>) -> Fill<V, State, Action> {
    Fill {
        child,
        mode: peniko::Fill::NonZero,
        brush: brush.into(),
        brush_transform: None,
        phantom: PhantomData,
    }
}

pub fn stroke<State, Action, V>(
    child: V,
    brush: impl Into<Brush>,
    style: kurbo::Stroke,
) -> Stroke<V, State, Action> {
    Stroke {
        child,
        style,
        brush: brush.into(),
        brush_transform: None,
        phantom: PhantomData,
    }
}

impl<V, State, Action> Fill<V, State, Action> {
    pub fn mode(mut self, mode: peniko::Fill) -> Self {
        self.mode = mode;
        self
    }

    pub fn brush_transform(mut self, brush_transform: Affine) -> Self {
        self.brush_transform = Some(brush_transform);
        self
    }
}

impl<V, State, Action> Stroke<V, State, Action> {
    pub fn brush_transform(mut self, brush_transform: Affine) -> Self {
        self.brush_transform = Some(brush_transform);
        self
    }
}

pub trait GraphicsExt<State, Action>: GraphicsView<State, Action> + Sized {
    fn transform(self, affine: Affine) -> Transform<Self, State, Action> {
        transform(self, affine)
    }

    fn fill(self, brush: impl Into<Brush>) -> Fill<Self, State, Action> {
        fill(self, brush)
    }

    fn stroke(self, brush: impl Into<Brush>, style: kurbo::Stroke) -> Stroke<Self, State, Action> {
        stroke(self, brush, style)
    }

    fn into_any_board(self) -> AnyBoardChild<State, Action>
    where
        Self: AnyView<State, Action, ViewCtx, Pod<KurboShape>> + Send + Sync + 'static,
    {
        AnyBoardChild::Graphics(Box::new(self))
    }
}

impl<State: 'static, Action: 'static, V: GraphicsView<State, Action>> GraphicsExt<State, Action>
    for V
{
}

impl<V, State, Action> ViewMarker for Transform<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for Transform<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: GraphicsView<State, Action>,
{
    type ViewState = V::ViewState;
    type Element = Pod<KurboShape>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = self.child.build(ctx);
        element
            .inner
            .as_mut()
            .unwrap()
            .set_transform(self.transform);
        (element, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);
        if self.transform != prev.transform {
            element.set_transform(self.transform);
            ctx.mark_changed();
        }
        element
    }

    fn teardown(
        &self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(child_state, ctx, element);
    }

    fn message(
        &self,
        child_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(child_state, id_path, message, app_state)
    }
}

impl<V, State, Action> ViewMarker for Fill<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for Fill<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: GraphicsView<State, Action>,
{
    type ViewState = V::ViewState;
    type Element = Pod<KurboShape>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = self.child.build(ctx);
        element.inner.as_mut().unwrap().set_fill_mode(self.mode);
        element
            .inner
            .as_mut()
            .unwrap()
            .set_fill_brush(self.brush.clone());
        element
            .inner
            .as_mut()
            .unwrap()
            .set_fill_brush_transform(self.brush_transform);
        (element, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);
        {
            if self.mode != prev.mode {
                element.set_fill_mode(self.mode);
                ctx.mark_changed();
            }
            if self.brush != prev.brush {
                element.set_fill_brush(self.brush.clone());
                ctx.mark_changed();
            }
            if self.brush_transform != prev.brush_transform {
                element.set_fill_brush_transform(self.brush_transform);
                ctx.mark_changed();
            }
        }
        element
    }

    fn teardown(
        &self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(child_state, ctx, element);
    }

    fn message(
        &self,
        child_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(child_state, id_path, message, app_state)
    }
}

impl<V, State, Action> ViewMarker for Stroke<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for Stroke<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: GraphicsView<State, Action>,
{
    type ViewState = V::ViewState;
    type Element = Pod<KurboShape>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = self.child.build(ctx);
        element
            .inner
            .as_mut()
            .unwrap()
            .set_stroke_style(self.style.clone());
        element
            .inner
            .as_mut()
            .unwrap()
            .set_stroke_brush(self.brush.clone());
        element
            .inner
            .as_mut()
            .unwrap()
            .set_stroke_brush_transform(self.brush_transform);
        (element, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);
        {
            if self.style.width != prev.style.width
                || self.style.join != prev.style.join
                || self.style.miter_limit != prev.style.miter_limit
                || self.style.start_cap != prev.style.start_cap
                || self.style.end_cap != prev.style.end_cap
                || self.style.dash_pattern != prev.style.dash_pattern
                || self.style.dash_offset != prev.style.dash_offset
            {
                element.set_stroke_style(self.style.clone());
                ctx.mark_changed();
            }
            if self.brush != prev.brush {
                element.set_stroke_brush(self.brush.clone());
                ctx.mark_changed();
            }
            if self.brush_transform != prev.brush_transform {
                element.set_stroke_brush_transform(self.brush_transform);
                ctx.mark_changed();
            }
        }
        element
    }

    fn teardown(
        &self,
        child_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(child_state, ctx, element);
    }

    fn message(
        &self,
        child_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(child_state, id_path, message, app_state)
    }
}
