// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in a one-dimensional array.

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace_span, warn, Span};
use vello::kurbo::{
    self, Affine, Arc, BezPath, Circle, CircleSegment, CubicBez, Ellipse, Line, PathEl, PathSeg,
    QuadBez, RoundedRect, Shape as _, Stroke, Vec2,
};
use vello::peniko::{Brush, Fill};
use vello::Scene;

use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Rect, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

/// A container with absolute positioning layout.
pub struct Board {
    children: Vec<Child>,
    origin: Point,
    scale: Vec2,
}

/// Parameters for an item in a [`Board`] container.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BoardParams {
    origin: Point,
    size: Size,
}

pub struct Shape {
    shape: KurboShape,
    transform: Affine,
    fill_style: Fill,
    fill_brush: Brush,
    fill_brush_transform: Option<Affine>,
    stroke_style: Stroke,
    stroke_brush: Brush,
    stroke_brush_transform: Option<Affine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KurboShape {
    PathSeg(PathSeg),
    Arc(Arc),
    BezPath(BezPath),
    Circle(Circle),
    CircleSegment(CircleSegment),
    CubicBez(CubicBez),
    Ellipse(Ellipse),
    Line(Line),
    QuadBez(QuadBez),
    Rect(Rect),
    RoundedRect(RoundedRect),
}

// --- MARK: IMPL BOARD ---
impl Board {
    /// Create a new Board oriented with viewport origin set to (0, 0) and scale (1, 1).
    pub fn new() -> Self {
        Board {
            children: Vec::new(),
            origin: Point::ZERO,
            scale: Vec2::new(1., 1.),
        }
    }

    /// Builder-style method to add a positioned child to the container.
    pub fn with_child_pod(
        mut self,
        widget: WidgetPod<Box<dyn Widget>>,
        params: impl Into<BoardParams>,
    ) -> Self {
        // TODO - dedup?
        self.children.push(Child::Widget(widget, params.into()));
        self
    }

    /// Builder-style method to add a Kurbo shape to the container.
    pub fn with_shape_child(mut self, shape: Shape) -> Self {
        self.children
            .push(Child::Shape(WidgetPod::new(Box::new(shape))));
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

// --- MARK: IMPL SHAPE ---
impl Shape {
    pub fn new(shape: impl Into<KurboShape>) -> Self {
        Shape {
            shape: shape.into(),
            transform: Default::default(),
            fill_style: Fill::NonZero,
            fill_brush: Default::default(),
            fill_brush_transform: Default::default(),
            stroke_style: Default::default(),
            stroke_brush: Default::default(),
            stroke_brush_transform: Default::default(),
        }
    }

    pub fn shape(&self) -> &KurboShape {
        &self.shape
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    pub fn set_fill_style(&mut self, fill_style: Fill) {
        self.fill_style = fill_style;
    }

    pub fn set_fill_brush(&mut self, fill_brush: Brush) {
        self.fill_brush = fill_brush;
    }

    pub fn set_fill_brush_transform(&mut self, fill_brush_transform: Option<Affine>) {
        self.fill_brush_transform = fill_brush_transform;
    }

    pub fn set_stroke_style(&mut self, stroke_style: Stroke) {
        self.stroke_style = stroke_style;
    }

    pub fn set_stroke_brush(&mut self, stroke_brush: Brush) {
        self.stroke_brush = stroke_brush;
    }

    pub fn set_stroke_brush_transform(&mut self, stroke_brush_transform: Option<Affine>) {
        self.stroke_brush_transform = stroke_brush_transform;
    }
}

// --- MARK: WIDGETMUT---
impl<'a> WidgetMut<'a, Board> {
    /// Set the origin.
    pub fn set_origin(&mut self, origin: Point) {
        self.widget.origin = origin;
        self.ctx.request_layout();
    }

    /// Set the scale.
    pub fn set_scale(&mut self, scale: Vec2) {
        self.widget.scale = scale;
        self.ctx.request_layout();
    }

    /// Add a positioned child widget.
    pub fn add_child(&mut self, child: impl Widget, params: impl Into<BoardParams>) {
        let child = Child::Widget(WidgetPod::new(Box::new(child)), params.into());

        self.widget.children.push(child);
        self.ctx.children_changed();
    }

    /// Add a Kurbo shape.
    pub fn add_shape_child(&mut self, shape: Shape) {
        self.widget
            .children
            .push(Child::Shape(WidgetPod::new(Box::new(shape))));
        self.ctx.children_changed();
    }

    pub fn insert_child(&mut self, idx: usize, child: impl Widget, params: impl Into<BoardParams>) {
        self.insert_child_pod(idx, WidgetPod::new(Box::new(child)), params);
    }

    pub fn insert_child_pod(
        &mut self,
        idx: usize,
        child: WidgetPod<Box<dyn Widget>>,
        params: impl Into<BoardParams>,
    ) {
        let child = Child::Widget(child, params.into());
        self.widget.children.insert(idx, child);
        self.ctx.children_changed();
    }

    pub fn insert_shape_child(&mut self, idx: usize, shape: Shape) {
        self.widget
            .children
            .insert(idx, Child::Shape(WidgetPod::new(Box::new(shape))));
        self.ctx.children_changed();
    }

    pub fn remove_child(&mut self, idx: usize) {
        let (Child::Widget(widget, _) | Child::Shape(widget)) = self.widget.children.remove(idx);
        self.ctx.remove_child(widget);
        self.ctx.request_layout();
    }

    // FIXME - Remove Box
    pub fn child_mut(&mut self, idx: usize) -> WidgetMut<'_, Box<dyn Widget>> {
        let (Child::Widget(widget, _) | Child::Shape(widget)) = &mut self.widget.children[idx];
        self.ctx.get_mut(widget)
    }

    /// Updates the position parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_board_params(&mut self, idx: usize, new_params: impl Into<BoardParams>) {
        if let Child::Widget(_, params) = &mut self.widget.children[idx] {
            *params = new_params.into();
            self.ctx.children_changed();
        }
    }

    pub fn clear(&mut self) {
        if !self.widget.children.is_empty() {
            self.ctx.request_layout();

            for child in self.widget.children.drain(..) {
                if let Child::Widget(widget, _) = child {
                    self.ctx.remove_child(widget);
                }
            }
        }
    }
}

impl<'a> WidgetMut<'a, Shape> {
    pub fn set_shape(&mut self, shape: KurboShape) {
        self.widget.shape = shape;
        self.ctx.request_layout();
        self.ctx.request_paint();
        self.ctx.request_accessibility_update();
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.widget.transform = transform;
        self.ctx.request_paint();
    }

    pub fn set_fill_style(&mut self, fill_style: Fill) {
        self.widget.fill_style = fill_style;
        self.ctx.request_paint();
    }

    pub fn set_fill_brush(&mut self, fill_brush: Brush) {
        self.widget.fill_brush = fill_brush;
        self.ctx.request_paint();
    }

    pub fn set_fill_brush_transform(&mut self, fill_brush_transform: Option<Affine>) {
        self.widget.fill_brush_transform = fill_brush_transform;
        self.ctx.request_paint();
    }

    pub fn set_stroke_style(&mut self, stroke_style: Stroke) {
        self.widget.stroke_style = stroke_style;
        self.ctx.request_paint();
    }

    pub fn set_stroke_brush(&mut self, stroke_brush: Brush) {
        self.widget.stroke_brush = stroke_brush;
        self.ctx.request_paint();
    }

    pub fn set_stroke_brush_transform(&mut self, stroke_brush_transform: Option<Affine>) {
        self.widget.stroke_brush_transform = stroke_brush_transform;
        self.ctx.request_paint();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Board {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.lifecycle(ctx, event);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        bc.debug_check("Board");

        for child in &mut self.children {
            match child {
                Child::Widget(widget, params) => {
                    if ctx.child_needs_layout(widget) {
                        ctx.run_layout(widget, &BoxConstraints::tight(params.size));
                    } else {
                        ctx.skip_layout(widget);
                    };
                    ctx.place_child(widget, self.origin + params.origin.to_vec2());
                }
                Child::Shape(shape) => {
                    let raw_widget = ctx.get_raw_ref(shape);
                    let params = BoardParams::from(
                        raw_widget
                            .widget()
                            .as_ref()
                            .as_dyn_any()
                            .downcast_ref::<Shape>()
                            .unwrap()
                            .shape()
                            .bounding_box(),
                    );
                    if ctx.child_needs_layout(shape) {
                        ctx.run_layout(shape, &BoxConstraints::tight(params.size));
                    } else {
                        ctx.skip_layout(shape);
                    };
                    ctx.place_child(shape, self.origin);
                }
            }
        }

        bc.max()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Board")
    }
}

impl Widget for Shape {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}
    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.shape.bounding_box().size();
        if !bc.contains(size) {
            warn!("The shape is oversized");
        }
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, scene: &mut Scene) {
        scene.fill(
            self.fill_style,
            self.transform,
            &self.fill_brush,
            self.fill_brush_transform,
            &self.shape,
        );
        scene.stroke(
            &self.stroke_style,
            self.transform,
            &self.stroke_brush,
            self.stroke_brush_transform,
            &self.shape,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::GraphicsSymbol
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Shape")
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

enum Child {
    Widget(WidgetPod<Box<dyn Widget>>, BoardParams),
    Shape(WidgetPod<Box<dyn Widget>>),
}

impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<Box<dyn Widget>>> {
        match self {
            Child::Widget(widget, _) | Child::Shape(widget) => Some(widget),
        }
    }
    fn widget(&self) -> Option<&WidgetPod<Box<dyn Widget>>> {
        match self {
            Child::Widget(widget, _) | Child::Shape(widget) => Some(widget),
        }
    }
}

macro_rules! for_all_variants {
    ($self:expr; $i:ident => $e:expr) => {
        match $self {
            Self::PathSeg($i) => $e,
            Self::Arc($i) => $e,
            Self::BezPath($i) => $e,
            Self::Circle($i) => $e,
            Self::CircleSegment($i) => $e,
            Self::CubicBez($i) => $e,
            Self::Ellipse($i) => $e,
            Self::Line($i) => $e,
            Self::QuadBez($i) => $e,
            Self::Rect($i) => $e,
            Self::RoundedRect($i) => $e,
        }
    };
}

impl kurbo::Shape for KurboShape {
    type PathElementsIter<'iter> = PathElementsIter<'iter>;

    fn path_elements(&self, tolerance: f64) -> Self::PathElementsIter<'_> {
        match self {
            Self::PathSeg(i) => PathElementsIter::PathSeg(i.path_elements(tolerance)),
            Self::Arc(i) => PathElementsIter::Arc(i.path_elements(tolerance)),
            Self::BezPath(i) => PathElementsIter::BezPath(i.path_elements(tolerance)),
            Self::Circle(i) => PathElementsIter::Circle(i.path_elements(tolerance)),
            Self::CircleSegment(i) => PathElementsIter::CircleSegment(i.path_elements(tolerance)),
            Self::CubicBez(i) => PathElementsIter::CubicBez(i.path_elements(tolerance)),
            Self::Ellipse(i) => PathElementsIter::Ellipse(i.path_elements(tolerance)),
            Self::Line(i) => PathElementsIter::Line(i.path_elements(tolerance)),
            Self::QuadBez(i) => PathElementsIter::QuadBez(i.path_elements(tolerance)),
            Self::Rect(i) => PathElementsIter::Rect(i.path_elements(tolerance)),
            Self::RoundedRect(i) => PathElementsIter::RoundedRect(i.path_elements(tolerance)),
        }
    }

    fn area(&self) -> f64 {
        for_all_variants!(self; i => i.area())
    }

    fn perimeter(&self, accuracy: f64) -> f64 {
        for_all_variants!(self; i => i.perimeter(accuracy))
    }

    fn winding(&self, pt: Point) -> i32 {
        for_all_variants!(self; i => i.winding(pt))
    }

    fn bounding_box(&self) -> Rect {
        for_all_variants!(self; i => i.bounding_box())
    }

    fn to_path(&self, tolerance: f64) -> BezPath {
        for_all_variants!(self; i => i.to_path(tolerance))
    }

    fn into_path(self, tolerance: f64) -> BezPath {
        for_all_variants!(self; i => i.into_path(tolerance))
    }

    fn contains(&self, pt: Point) -> bool {
        for_all_variants!(self; i => i.contains(pt))
    }

    fn as_line(&self) -> Option<Line> {
        for_all_variants!(self; i => i.as_line())
    }

    fn as_rect(&self) -> Option<Rect> {
        for_all_variants!(self; i => i.as_rect())
    }

    fn as_rounded_rect(&self) -> Option<RoundedRect> {
        for_all_variants!(self; i => i.as_rounded_rect())
    }

    fn as_circle(&self) -> Option<Circle> {
        for_all_variants!(self; i => i.as_circle())
    }

    fn as_path_slice(&self) -> Option<&[PathEl]> {
        for_all_variants!(self; i => i.as_path_slice())
    }
}

macro_rules! impl_from_shape {
    ($t:ident) => {
        impl From<kurbo::$t> for KurboShape {
            fn from(value: kurbo::$t) -> Self {
                KurboShape::$t(value)
            }
        }
    };
}

impl_from_shape!(PathSeg);
impl_from_shape!(Arc);
impl_from_shape!(BezPath);
impl_from_shape!(Circle);
impl_from_shape!(CircleSegment);
impl_from_shape!(CubicBez);
impl_from_shape!(Ellipse);
impl_from_shape!(Line);
impl_from_shape!(QuadBez);
impl_from_shape!(Rect);
impl_from_shape!(RoundedRect);

pub enum PathElementsIter<'i> {
    PathSeg(<PathSeg as kurbo::Shape>::PathElementsIter<'i>),
    Arc(<Arc as kurbo::Shape>::PathElementsIter<'i>),
    BezPath(<BezPath as kurbo::Shape>::PathElementsIter<'i>),
    Circle(<Circle as kurbo::Shape>::PathElementsIter<'i>),
    CircleSegment(<CircleSegment as kurbo::Shape>::PathElementsIter<'i>),
    CubicBez(<CubicBez as kurbo::Shape>::PathElementsIter<'i>),
    Ellipse(<Ellipse as kurbo::Shape>::PathElementsIter<'i>),
    Line(<Line as kurbo::Shape>::PathElementsIter<'i>),
    QuadBez(<QuadBez as kurbo::Shape>::PathElementsIter<'i>),
    Rect(<Rect as kurbo::Shape>::PathElementsIter<'i>),
    RoundedRect(<RoundedRect as kurbo::Shape>::PathElementsIter<'i>),
}

impl<'i> Iterator for PathElementsIter<'i> {
    type Item = PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        for_all_variants!(self; i => i.next())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use vello::{kurbo::Circle, peniko::Brush};

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Button;

    #[test]
    fn kurbo_shape_circle() {
        let mut widget = Shape::new(Circle::new((50., 50.), 30.));
        widget.set_fill_brush(Brush::Solid(vello::peniko::Color::CHARTREUSE));
        widget.set_stroke_style(Stroke::new(2.).with_dashes(0., [2., 1.]));
        widget.set_stroke_brush(Brush::Solid(vello::peniko::Color::PALE_VIOLET_RED));

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "kurbo_shape_circle");
    }

    #[test]
    fn board_absolute_placement_snapshots() {
        let widget = Board::new()
            .with_child_pod(
                WidgetPod::new(Box::new(Button::new("hello"))),
                Rect::new(10., 10., 60., 40.),
            )
            .with_child_pod(
                WidgetPod::new(Box::new(Button::new("world"))),
                Rect::new(30., 30., 80., 60.),
            );

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "absolute_placement");
    }

    #[test]
    fn board_shape_placement_snapshots() {
        let mut shape = Shape::new(Circle::new((70., 50.), 30.));
        shape.set_fill_brush(Brush::Solid(vello::peniko::Color::NAVY));
        shape.set_stroke_style(Stroke::new(2.).with_dashes(0., [2., 1.]));
        shape.set_stroke_brush(Brush::Solid(vello::peniko::Color::PALE_VIOLET_RED));
        let widget = Board::new()
            .with_child_pod(
                WidgetPod::new(Box::new(Button::new("hello"))),
                Rect::new(10., 10., 60., 40.),
            )
            .with_shape_child(shape);

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "shape_placement");
    }
}
