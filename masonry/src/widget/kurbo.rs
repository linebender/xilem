// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget representing a `kurbo::Shape`.

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace_span, warn, Span};
use vello::kurbo::{
    self, Affine, Arc, BezPath, Circle, CircleSegment, CubicBez, Ellipse, Line, PathEl, PathSeg,
    QuadBez, RoundedRect, Shape, Stroke,
};
use vello::peniko::{Brush, Fill};
use vello::Scene;

use crate::widget::{SvgElement, WidgetMut, WidgetPod};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Rect, Size, StatusChange, TextEvent, Widget, WidgetId,
};

/// A widget representing a `kurbo::Shape`.
pub struct KurboShape {
    shape: ConcreteShape,
    transform: Affine,
    fill: Option<FillParams>,
    stroke: Option<StrokeParams>,
}

struct FillParams {
    mode: Fill,
    brush: Brush,
    brush_transform: Option<Affine>,
}

#[derive(Default)]
struct StrokeParams {
    style: Stroke,
    brush: Brush,
    brush_transform: Option<Affine>,
}

/// A concrete type for all built-in `kurbo::Shape`s.
// TODO: Adopt `kurbo::ConcreteShape` once https://github.com/linebender/kurbo/pull/331 merges
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteShape {
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

// --- MARK: IMPL KURBOSHAPE ---
impl KurboShape {
    pub fn new(shape: impl Into<ConcreteShape>) -> Self {
        KurboShape {
            shape: shape.into(),
            transform: Default::default(),
            fill: None,
            stroke: None,
        }
    }

    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    pub fn set_fill_mode(&mut self, fill_mode: Fill) {
        self.fill.get_or_insert_with(Default::default).mode = fill_mode;
    }

    pub fn set_fill_brush(&mut self, fill_brush: Brush) {
        self.fill.get_or_insert_with(Default::default).brush = fill_brush;
    }

    pub fn set_fill_brush_transform(&mut self, fill_brush_transform: Option<Affine>) {
        self.fill
            .get_or_insert_with(Default::default)
            .brush_transform = fill_brush_transform;
    }

    pub fn set_stroke_style(&mut self, stroke_style: Stroke) {
        self.stroke.get_or_insert_with(Default::default).style = stroke_style;
    }

    pub fn set_stroke_brush(&mut self, stroke_brush: Brush) {
        self.stroke.get_or_insert_with(Default::default).brush = stroke_brush;
    }

    pub fn set_stroke_brush_transform(&mut self, stroke_brush_transform: Option<Affine>) {
        self.stroke
            .get_or_insert_with(Default::default)
            .brush_transform = stroke_brush_transform;
    }
}

// MARK: WIDGETMUT
impl<'a> WidgetMut<'a, KurboShape> {
    pub fn set_shape(&mut self, shape: ConcreteShape) {
        self.widget.shape = shape;
        self.ctx.request_layout();
        self.ctx.request_paint();
        self.ctx.request_accessibility_update();
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.widget.transform = transform;
        self.ctx.request_paint();
    }

    pub fn set_fill_mode(&mut self, fill_mode: Fill) {
        self.widget.fill.get_or_insert_with(Default::default).mode = fill_mode;
        self.ctx.request_paint();
    }

    pub fn set_fill_brush(&mut self, fill_brush: Brush) {
        self.widget.fill.get_or_insert_with(Default::default).brush = fill_brush;
        self.ctx.request_paint();
    }

    pub fn set_fill_brush_transform(&mut self, fill_brush_transform: Option<Affine>) {
        self.widget
            .fill
            .get_or_insert_with(Default::default)
            .brush_transform = fill_brush_transform;
        self.ctx.request_paint();
    }

    pub fn set_stroke_style(&mut self, stroke_style: Stroke) {
        self.widget
            .stroke
            .get_or_insert_with(Default::default)
            .style = stroke_style;
        self.ctx.request_paint();
    }

    pub fn set_stroke_brush(&mut self, stroke_brush: Brush) {
        self.widget
            .stroke
            .get_or_insert_with(Default::default)
            .brush = stroke_brush;
        self.ctx.request_paint();
    }

    pub fn set_stroke_brush_transform(&mut self, stroke_brush_transform: Option<Affine>) {
        self.widget
            .stroke
            .get_or_insert_with(Default::default)
            .brush_transform = stroke_brush_transform;
        self.ctx.request_paint();
    }
}

// MARK: IMPL WIDGET
impl Widget for KurboShape {
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
        let transform = self
            .transform
            .then_translate(-self.shape.bounding_box().origin().to_vec2());
        if let Some(FillParams {
            mode,
            brush,
            brush_transform,
        }) = &self.fill
        {
            scene.fill(*mode, transform, brush, *brush_transform, &self.shape);
        }
        if let Some(StrokeParams {
            style,
            brush,
            brush_transform,
        }) = &self.stroke
        {
            scene.stroke(style, transform, brush, *brush_transform, &self.shape);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GraphicsSymbol
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("KurboShape")
    }
}

impl SvgElement for KurboShape {
    fn origin(&self) -> Point {
        self.shape.bounding_box().origin()
    }

    fn size(&self) -> Size {
        self.shape.bounding_box().size()
    }

    fn set_origin(&mut self, _: Point) {
        panic!("a shape does not support setting its origin after creation")
    }

    fn set_size(&mut self, _: Size) {
        panic!("a shape does not support setting its size after creation")
    }
}

// --- MARK: OTHER IMPLS ---
impl Default for FillParams {
    fn default() -> Self {
        Self {
            mode: Fill::NonZero,
            brush: Default::default(),
            brush_transform: Default::default(),
        }
    }
}

impl WidgetPod<KurboShape> {
    pub fn svg_boxed(self) -> WidgetPod<Box<dyn SvgElement>> {
        let id = self.id();
        WidgetPod::new_with_id(Box::new(self.inner().unwrap()), id)
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

impl Shape for ConcreteShape {
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
        impl From<kurbo::$t> for ConcreteShape {
            fn from(value: kurbo::$t) -> Self {
                ConcreteShape::$t(value)
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
    PathSeg(<PathSeg as Shape>::PathElementsIter<'i>),
    Arc(<Arc as Shape>::PathElementsIter<'i>),
    BezPath(<BezPath as Shape>::PathElementsIter<'i>),
    Circle(<Circle as Shape>::PathElementsIter<'i>),
    CircleSegment(<CircleSegment as Shape>::PathElementsIter<'i>),
    CubicBez(<CubicBez as Shape>::PathElementsIter<'i>),
    Ellipse(<Ellipse as Shape>::PathElementsIter<'i>),
    Line(<Line as Shape>::PathElementsIter<'i>),
    QuadBez(<QuadBez as Shape>::PathElementsIter<'i>),
    Rect(<Rect as Shape>::PathElementsIter<'i>),
    RoundedRect(<RoundedRect as Shape>::PathElementsIter<'i>),
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

    #[test]
    fn kurbo_shape_circle() {
        let mut widget = KurboShape::new(Circle::new((50., 50.), 30.));
        widget.set_fill_brush(Brush::Solid(vello::peniko::Color::CHARTREUSE));
        widget.set_stroke_style(Stroke::new(2.).with_dashes(0., [2., 1.]));
        widget.set_stroke_brush(Brush::Solid(vello::peniko::Color::PALE_VIOLET_RED));

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "kurbo_shape_circle");
    }

    // TODO: add test for KurboShape in Flex
}
