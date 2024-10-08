// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget with predefined size.

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, warn, Span};
use vello::kurbo::{Affine, RoundedRectRadii};
use vello::peniko::{Brush, Color, Fill};
use vello::Scene;

use crate::paint_scene_helpers::stroke;
use crate::widget::{WidgetMut, WidgetPod};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, Point,
    PointerEvent, RegisterCtx, Size, StatusChange, TextEvent, Widget, WidgetId,
};

// FIXME - Improve all doc in this module ASAP.

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
/// If not given a child, `SizedBox` will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct SizedBox {
    child: Option<WidgetPod<Box<dyn Widget>>>,
    width: Option<f64>,
    height: Option<f64>,
    background: Option<Brush>,
    border: Option<BorderStyle>,
    corner_radius: RoundedRectRadii,
}

// --- MARK: BUILDERS ---
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

    /// Construct container with child in a pod, and both width and height not set.
    pub fn new_pod(child: WidgetPod<Box<dyn Widget>>) -> Self {
        Self {
            child: Some(child),
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
    /// [`expand_height`]: Self::expand_height
    /// [`expand_width`]: Self::expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self.height = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(f64::INFINITY);
        self
    }

    /// Builder-style method for setting the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`Brush`];
    /// notably, it can be any [`Color`], any gradient, or an [`Image`].
    ///
    /// [`Image`]: vello::peniko::Image
    pub fn background(mut self, brush: impl Into<Brush>) -> Self {
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

    /// Set the width directly. Intended for toolkits abstracting over `SizedBox`
    pub fn raw_width(mut self, value: Option<f64>) -> Self {
        self.width = value;
        self
    }

    /// Set the height directly. Intended for toolkits abstracting over `SizedBox`
    pub fn raw_height(mut self, value: Option<f64>) -> Self {
        self.height = value;
        self
    }

    // TODO - child()
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, SizedBox> {
    pub fn set_child(&mut self, child: impl Widget) {
        if let Some(child) = self.widget.child.take() {
            self.ctx.remove_child(child);
        }
        self.widget.child = Some(WidgetPod::new(child).boxed());
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn remove_child(&mut self) {
        if let Some(child) = self.widget.child.take() {
            self.ctx.remove_child(child);
        }
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
    /// This can be passed anything which can be represented by a [`Brush`];
    /// notably, it can be any [`Color`], any gradient, or an [`Image`].
    ///
    /// [`Image`]: vello::peniko::Image
    pub fn set_background(&mut self, brush: impl Into<Brush>) {
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

// --- MARK: INTERNALS ---
impl SizedBox {
    fn child_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let max_width = match self.width {
            Some(width) => width.min(bc.max().width),
            None => bc.max().width,
        };

        let max_height = match self.height {
            Some(height) => height.min(bc.max().height),
            None => bc.max().height,
        };

        BoxConstraints::new(Size::new(max_width, max_height))
    }

    #[allow(dead_code)]
    pub(crate) fn width_and_height(&self) -> (Option<f64>, Option<f64>) {
        (self.width, self.height)
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for SizedBox {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
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
                size = ctx.run_layout(child, &child_bc);
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
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &*background,
                    Some(Affine::IDENTITY),
                    &panel,
                );
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
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        if let Some(child) = &self.child {
            smallvec![child.id()]
        } else {
            smallvec![]
        }
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("SizedBox")
    }
}

// --- Tests ---

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use vello::peniko::Gradient;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Label;

    #[test]
    fn no_width() {
        let expand = SizedBox::new(Label::new("hello!")).height(200.);
        let bc = BoxConstraints::new(Size::new(400., 400.));
        let child_bc = expand.child_constraints(&bc);
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
        assert_render_snapshot!(harness, "label_box_with_size");
    }

    #[test]
    fn label_box_with_solid_background() {
        let widget = SizedBox::new(Label::new("hello"))
            .width(40.0)
            .height(40.0)
            .background(Color::PLUM);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "label_box_with_solid_background");
    }

    #[test]
    fn empty_box_with_gradient_background() {
        let widget = SizedBox::empty()
            .width(40.)
            .height(40.)
            .rounded(20.)
            .border(Color::LIGHT_SKY_BLUE, 5.)
            .background(
                Gradient::new_sweep((30., 30.), 0., std::f32::consts::TAU).with_stops([
                    (0., Color::WHITE),
                    (0.25, Color::BLACK),
                    (0.5, Color::RED),
                    (0.75, Color::GREEN),
                    (1., Color::WHITE),
                ]),
            );

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "empty_box_with_gradient_background");
    }

    // TODO - add screenshot tests for different brush types
}
