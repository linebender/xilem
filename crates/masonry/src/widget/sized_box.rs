// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A widget with predefined size.

use std::f64::INFINITY;

use kurbo::Affine;
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, warn, Span};
use vello::peniko::{BlendMode, Color, Fill, Gradient};
use vello::Scene;

use crate::kurbo::RoundedRectRadii;
use crate::paint_scene_helpers::{fill_color, stroke};
use crate::widget::{WidgetId, WidgetMut, WidgetPod, WidgetRef};
use crate::{
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, PointerEvent,
    Size, StatusChange, TextEvent, Widget,
};

// FIXME - Improve all doc in this module ASAP.

/// Something that can be used as the background for a widget.
#[non_exhaustive]
#[allow(missing_docs)]
#[allow(clippy::type_complexity)]
pub enum BackgroundBrush {
    Color(Color),
    Gradient(Gradient),
    PainterFn(Box<dyn FnMut(&mut PaintCtx)>),
}

/// Something that can be used as the border for a widget.
struct BorderStyle {
    width: f64,
    color: Color,
}

// TODO - Have Widget type as generic argument
// TODO - Add Padding

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not
/// set, this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, SizedBox will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct SizedBox {
    child: Option<WidgetPod<Box<dyn Widget>>>,
    width: Option<f64>,
    height: Option<f64>,
    background: Option<BackgroundBrush>,
    border: Option<BorderStyle>,
    corner_radius: RoundedRectRadii,
}
crate::declare_widget!(SizedBoxMut, SizedBox);

impl SizedBox {
    /// Construct container with child, and both width and height not set.
    pub fn new(child: impl Widget) -> Self {
        Self {
            child: Some(WidgetPod::new(child).boxed()),
            width: None,
            height: None,
            background: None,
            border: None,
            corner_radius: RoundedRectRadii::from_single_radius(0.0),
        }
    }

    /// Construct container with child, and both width and height not set.
    pub fn new_with_id(child: impl Widget, id: WidgetId) -> Self {
        Self {
            child: Some(WidgetPod::new_with_id(child, id).boxed()),
            width: None,
            height: None,
            background: None,
            border: None,
            corner_radius: RoundedRectRadii::from_single_radius(0.0),
        }
    }

    /// Construct container without child, and both width and height not set.
    ///
    /// If the widget is unchanged, it will render nothing, which can be useful if you want to draw a
    /// widget some of the time.
    #[doc(alias = "null")]
    pub fn empty() -> Self {
        Self {
            child: None,
            width: None,
            height: None,
            background: None,
            border: None,
            corner_radius: RoundedRectRadii::from_single_radius(0.0),
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(INFINITY);
        self.height = Some(INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(INFINITY);
        self
    }

    /// Builder-style method for setting the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], any gradient, or a fully custom painter `FnMut`.
    pub fn background(mut self, brush: impl Into<BackgroundBrush>) -> Self {
        self.background = Some(brush.into());
        self
    }

    /// Builder-style method for painting a border around the widget with a color and width.
    pub fn border(mut self, color: impl Into<Color>, width: impl Into<f64>) -> Self {
        self.border = Some(BorderStyle {
            color: color.into(),
            width: width.into(),
        });
        self
    }

    /// Builder style method for rounding off corners of this container by setting a corner radius
    pub fn rounded(mut self, radius: impl Into<RoundedRectRadii>) -> Self {
        self.corner_radius = radius.into();
        self
    }

    // TODO - child()
}

impl<'a> SizedBoxMut<'a> {
    pub fn set_child(&mut self, child: impl Widget) {
        self.widget.child = Some(WidgetPod::new(child).boxed());
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn remove_child(&mut self) {
        self.widget.child = None;
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    /// Set container's width.
    pub fn set_width(&mut self, width: f64) {
        self.widget.width = Some(width);
        self.ctx.request_layout();
    }

    /// Set container's height.
    pub fn set_height(&mut self, height: f64) {
        self.widget.height = Some(height);
        self.ctx.request_layout();
    }

    /// Set container's width.
    pub fn unset_width(&mut self) {
        self.widget.width = None;
        self.ctx.request_layout();
    }

    /// Set container's height.
    pub fn unset_height(&mut self) {
        self.widget.height = None;
        self.ctx.request_layout();
    }

    /// Set the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], any gradient, or a fully custom painter `FnMut`.
    pub fn set_background(&mut self, brush: impl Into<BackgroundBrush>) {
        self.widget.background = Some(brush.into());
        self.ctx.request_paint();
    }

    /// Clears background.
    pub fn clear_background(&mut self) {
        self.widget.background = None;
        self.ctx.request_paint();
    }

    /// Paint a border around the widget with a color and width.
    pub fn set_border(&mut self, color: impl Into<Color>, width: impl Into<f64>) {
        self.widget.border = Some(BorderStyle {
            color: color.into(),
            width: width.into(),
        });
        self.ctx.request_layout();
    }

    /// Clears border.
    pub fn clear_border(&mut self) {
        self.widget.border = None;
        self.ctx.request_layout();
    }

    /// Round off corners of this container by setting a corner radius
    pub fn set_rounded(&mut self, radius: impl Into<RoundedRectRadii>) {
        self.widget.corner_radius = radius.into();
        self.ctx.request_paint();
    }

    // TODO - Doc
    pub fn child_mut(&mut self) -> Option<WidgetMut<'_, Box<dyn Widget>>> {
        let child = self.widget.child.as_mut()?;
        Some(self.ctx.get_mut(child))
    }
}

impl SizedBox {
    fn child_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match self.width {
            Some(width) => {
                let w = width.max(bc.min().width).min(bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match self.height {
            Some(height) => {
                let h = height.max(bc.min().height).min(bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            Size::new(min_width, min_height),
            Size::new(max_width, max_height),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn width_and_height(&self) -> (Option<f64>, Option<f64>) {
        (self.width, self.height)
    }
}

impl Widget for SizedBox {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        if let Some(ref mut child) = self.child {
            child.on_pointer_event(ctx, event);
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        if let Some(ref mut child) = self.child {
            child.on_text_event(ctx, event);
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        if let Some(ref mut child) = self.child {
            child.lifecycle(ctx, event)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // Shrink constraints by border offset
        let border_width = match &self.border {
            Some(border) => border.width,
            None => 0.0,
        };

        let child_bc = self.child_constraints(bc);
        let child_bc = child_bc.shrink((2.0 * border_width, 2.0 * border_width));
        let origin = Point::new(border_width, border_width);

        let mut size;
        match self.child.as_mut() {
            Some(child) => {
                size = child.layout(ctx, &child_bc);
                ctx.place_child(child, origin);
                size = Size::new(
                    size.width + 2.0 * border_width,
                    size.height + 2.0 * border_width,
                );
            }
            None => size = bc.constrain((self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))),
        };

        // TODO - figure out paint insets
        // TODO - figure out baseline offset

        trace!("Computed size: {}", size);

        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }
        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let corner_radius = self.corner_radius;

        if let Some(background) = self.background.as_mut() {
            let panel = ctx.size().to_rounded_rect(corner_radius);

            trace_span!("paint background").in_scope(|| {
                scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &panel);
                background.paint(ctx, scene);
                scene.pop_layer();
            });
        }

        if let Some(border) = &self.border {
            let border_width = border.width;
            let border_rect = ctx
                .size()
                .to_rect()
                .inset(border_width / -2.0)
                .to_rounded_rect(corner_radius);
            stroke(scene, &border_rect, border.color, border_width);
        };

        if let Some(ref mut child) = self.child {
            child.paint(ctx, scene);
        }
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        if let Some(child) = &self.child {
            smallvec![child.as_dyn()]
        } else {
            smallvec![]
        }
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("SizedBox")
    }
}

// --- BackgroundBrush ---

impl BackgroundBrush {
    /// Draw this brush into a provided [`PaintCtx`].
    pub fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let bounds = ctx.size().to_rect();
        match self {
            Self::Color(color) => fill_color(scene, &bounds, *color),
            Self::Gradient(grad) => scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &*grad,
                Some(Affine::IDENTITY),
                &bounds,
            ),
            Self::PainterFn(painter) => painter(ctx),
        }
    }
}

impl From<Color> for BackgroundBrush {
    fn from(src: Color) -> BackgroundBrush {
        BackgroundBrush::Color(src)
    }
}

impl From<Gradient> for BackgroundBrush {
    fn from(src: Gradient) -> BackgroundBrush {
        BackgroundBrush::Gradient(src)
    }
}

impl<Painter: FnMut(&mut PaintCtx) + 'static> From<Painter> for BackgroundBrush {
    fn from(src: Painter) -> BackgroundBrush {
        BackgroundBrush::PainterFn(Box::new(src))
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Label;

    #[test]
    fn expand() {
        let expand = SizedBox::new(Label::new("hello!")).expand();
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(400., 400.,));
    }

    #[test]
    fn no_width() {
        let expand = SizedBox::new(Label::new("hello!")).height(200.);
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(0., 200.,));
        assert_eq!(child_bc.max(), Size::new(400., 200.,));
    }

    #[test]
    fn empty_box() {
        let widget = SizedBox::empty()
            .width(40.0)
            .height(40.0)
            .border(Color::BLUE, 5.0)
            .rounded(5.0);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "empty_box");
    }

    #[test]
    fn label_box_no_size() {
        let widget = SizedBox::new(Label::new("hello"))
            .border(Color::BLUE, 5.0)
            .rounded(5.0);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "label_box_no_size");
    }

    #[test]
    fn label_box_with_size() {
        let widget = SizedBox::new(Label::new("hello"))
            .width(40.0)
            .height(40.0)
            .border(Color::BLUE, 5.0)
            .rounded(5.0);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "label_box_no_size");
    }

    // TODO - add screenshot tests for different brush types
}
