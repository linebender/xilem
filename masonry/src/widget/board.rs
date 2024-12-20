// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in fixed positions.

use std::ops::{Deref as _, DerefMut as _};

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::Scene;

use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Rect, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

/// A container with absolute positioning layout.
pub struct Board {
    children: Vec<WidgetPod<Box<dyn SvgElement>>>,
}

/// Parameters for an item in a [`Board`] container.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BoardParams {
    pub origin: Point,
    pub size: Size,
}

/// Wrapper of a regular widget for use in [`Board`]
pub struct PositionedElement<W> {
    inner: W,
    params: BoardParams,
}

/// A trait representing a widget which knows its origin and size.
pub trait SvgElement: Widget {
    // Origin of the widget relative to its parent
    fn origin(&self) -> Point;
    // Size of the widget
    fn size(&self) -> Size;

    // Sets the origin of the widget relative to its parent
    fn set_origin(&mut self, origin: Point);
    // Sets the size of the widget
    fn set_size(&mut self, size: Size);
}

// --- MARK: IMPL BOARD ---
impl Board {
    /// Create a new empty Board.
    pub fn new() -> Self {
        Board {
            children: Vec::new(),
        }
    }

    /// Builder-style method to add a positioned child to the container.
    pub fn with_child_pod(mut self, child: WidgetPod<Box<dyn SvgElement>>) -> Self {
        self.children.push(child);
        self
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

// --- MARK: WIDGETMUT---
impl<'a> WidgetMut<'a, Board> {
    /// Add a positioned child widget.
    pub fn add_child(&mut self, child: impl SvgElement) {
        self.widget.children.push(WidgetPod::new(Box::new(child)));
        self.ctx.children_changed();
    }

    pub fn insert_child(&mut self, idx: usize, child: WidgetPod<Box<dyn SvgElement>>) {
        self.widget.children.insert(idx, child);
        self.ctx.children_changed();
    }

    pub fn remove_child(&mut self, idx: usize) {
        let widget = self.widget.children.remove(idx);
        self.ctx.remove_child(widget);
        self.ctx.request_layout();
    }

    pub fn child_mut(&mut self, idx: usize) -> WidgetMut<'_, Box<dyn SvgElement>> {
        self.ctx.get_mut(&mut self.widget.children[idx])
    }

    pub fn clear(&mut self) {
        if !self.widget.children.is_empty() {
            self.ctx.request_layout();

            for child in self.widget.children.drain(..) {
                self.ctx.remove_child(child);
            }
        }
    }
}

impl<'a, W: Widget> WidgetMut<'a, PositionedElement<W>> {
    pub fn inner_mut(&mut self) -> WidgetMut<'_, W> {
        WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget: &mut self.widget.inner,
        }
    }
}

impl<'a> WidgetMut<'a, Box<dyn SvgElement>> {
    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn try_downcast<W2: Widget>(&mut self) -> Option<WidgetMut<'_, W2>> {
        Some(WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget: self.widget.as_mut_any().downcast_mut()?,
        })
    }

    /// Downcasts to `WidgetMut` of concrete Widget type.
    ///
    /// ## Panics
    ///
    /// Panics if the downcast fails, with an error message that shows the
    /// discrepancy between the expected and actual types.
    pub fn downcast<W2: Widget>(&mut self) -> WidgetMut<'_, W2> {
        let w1_name = self.widget.type_name();
        match self.widget.as_mut_any().downcast_mut() {
            Some(widget) => WidgetMut {
                ctx: self.ctx.reborrow_mut(),
                widget,
            },
            None => {
                panic!(
                    "failed to downcast widget: expected widget of type `{}`, found `{}`",
                    std::any::type_name::<W2>(),
                    w1_name,
                );
            }
        }
    }
}

// --- MARK: IMPL WIDGET BOARD ---
impl Widget for Board {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}
    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(ctx, event);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        bc.debug_check("Board");

        for child in &mut self.children {
            let (size, origin) = {
                let child_ref = ctx.get_raw_ref(child);
                (child_ref.widget().size(), child_ref.widget().origin())
            };
            ctx.run_layout(child, &BoxConstraints::tight(size));
            ctx.place_child(child, origin);
        }

        bc.max()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children.iter().map(|child| child.id()).collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Board")
    }
}

// --- MARK: IMPL WIDGET POSITIONEDELEMENT ---
impl<W: Widget> Widget for PositionedElement<W> {
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        self.inner.on_status_change(ctx, event);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.inner.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        self.inner.layout(ctx, bc)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.inner.paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        self.inner.accessibility_role()
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        self.inner.accessibility(ctx);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.inner.children_ids()
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.inner.on_pointer_event(ctx, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.inner.on_text_event(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.inner.on_access_event(ctx, event);
    }

    fn compose(&mut self, ctx: &mut crate::ComposeCtx) {
        self.inner.compose(ctx);
    }

    fn skip_pointer(&self) -> bool {
        self.inner.skip_pointer()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.inner.get_debug_text()
    }

    fn get_cursor(&self) -> cursor_icon::CursorIcon {
        self.inner.get_cursor()
    }
}

impl<W: Widget> SvgElement for PositionedElement<W> {
    fn origin(&self) -> Point {
        self.params.origin
    }

    fn size(&self) -> Size {
        self.params.size
    }

    fn set_origin(&mut self, origin: Point) {
        self.params.origin = origin;
    }

    fn set_size(&mut self, size: Size) {
        self.params.size = size;
    }
}

// --- MARK: IMPL WIDGET SVGELEMENT ---
impl Widget for Box<dyn SvgElement> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.deref_mut().on_pointer_event(ctx, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.deref_mut().on_text_event(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.deref_mut().on_access_event(ctx, event);
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        self.deref_mut().on_status_change(ctx, event);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.deref_mut().lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        self.deref_mut().layout(ctx, bc)
    }

    fn compose(&mut self, ctx: &mut crate::ComposeCtx) {
        self.deref_mut().compose(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.deref_mut().paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        self.deref().accessibility_role()
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        self.deref_mut().accessibility(ctx);
    }

    fn type_name(&self) -> &'static str {
        self.deref().type_name()
    }

    fn short_type_name(&self) -> &'static str {
        self.deref().short_type_name()
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.deref().children_ids()
    }

    fn skip_pointer(&self) -> bool {
        self.deref().skip_pointer()
    }

    fn make_trace_span(&self) -> Span {
        self.deref().make_trace_span()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.deref().get_debug_text()
    }

    fn get_cursor(&self) -> cursor_icon::CursorIcon {
        self.deref().get_cursor()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self.deref().as_any()
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self.deref_mut().as_mut_any()
    }
}

impl SvgElement for Box<dyn SvgElement> {
    fn origin(&self) -> Point {
        self.deref().origin()
    }

    fn size(&self) -> Size {
        self.deref().size()
    }

    fn set_origin(&mut self, origin: Point) {
        self.deref_mut().set_origin(origin);
    }

    fn set_size(&mut self, size: Size) {
        self.deref_mut().set_size(size);
    }
}

// --- MARK: OTHER IMPLS---
impl BoardParams {
    /// Create a `BoardParams` with a specific `origin` and `size`.
    pub fn new(origin: impl Into<Point>, size: impl Into<Size>) -> Self {
        BoardParams {
            origin: origin.into(),
            size: size.into(),
        }
    }
}

impl From<Rect> for BoardParams {
    fn from(rect: Rect) -> Self {
        BoardParams {
            origin: rect.origin(),
            size: rect.size(),
        }
    }
}

impl<W: Widget> WidgetPod<W> {
    pub fn positioned(self, params: impl Into<BoardParams>) -> WidgetPod<PositionedElement<W>> {
        let id = self.id();
        WidgetPod::new_with_id(
            PositionedElement {
                inner: self.inner().unwrap(),
                params: params.into(),
            },
            id,
        )
    }
}

impl<W: Widget> WidgetPod<PositionedElement<W>> {
    pub fn svg_boxed(self) -> WidgetPod<Box<dyn SvgElement>> {
        let id = self.id();
        WidgetPod::new_with_id(Box::new(self.inner().unwrap()), id)
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use vello::kurbo::{Circle, Stroke};
    use vello::peniko::Brush;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::{Button, KurboShape};

    #[test]
    fn board_absolute_placement_snapshots() {
        let board = Board::new()
            .with_child_pod(
                WidgetPod::new(Button::new("hello"))
                    .positioned(Rect::new(10., 10., 60., 40.))
                    .svg_boxed(),
            )
            .with_child_pod(
                WidgetPod::new(Button::new("world"))
                    .positioned(Rect::new(30., 30., 80., 60.))
                    .svg_boxed(),
            );

        let mut harness = TestHarness::create(board);

        assert_render_snapshot!(harness, "absolute_placement");
    }

    #[test]
    fn board_shape_placement_snapshots() {
        let mut shape = KurboShape::new(Circle::new((70., 50.), 30.));
        shape.set_fill_brush(Brush::Solid(vello::peniko::Color::NAVY));
        shape.set_stroke_style(Stroke::new(2.).with_dashes(0., [2., 1.]));
        shape.set_stroke_brush(Brush::Solid(vello::peniko::Color::PALE_VIOLET_RED));

        let board = Board::new()
            .with_child_pod(
                WidgetPod::new(Button::new("hello"))
                    .positioned(Rect::new(10., 10., 60., 40.))
                    .svg_boxed(),
            )
            .with_child_pod(WidgetPod::new(Box::new(shape)));

        let mut harness = TestHarness::create(board);

        assert_render_snapshot!(harness, "shape_placement");
    }
}
