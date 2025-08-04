// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget with predefined size.

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span, warn};
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::types::Length;
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::util::{fill, include_screenshot, stroke};

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not
/// set, this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, `SizedBox` will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
///
#[doc = include_screenshot!("sized_box_label_box_with_outer_padding.png", "Box with blue border, pink background and a child label.")]
pub struct SizedBox {
    child: Option<WidgetPod<dyn Widget>>,
    width: Option<f64>,
    height: Option<f64>,
}

// --- MARK: BUILDERS
impl SizedBox {
    /// Construct container with child, and both width and height not set.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: Some(child.erased().to_pod()),
            width: None,
            height: None,
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
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = Some(width.value());
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = Some(height.value());
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
        // TODO - Using infinity in layout is a code smell.
        // Rewor these methods.
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
}

// --- MARK: WIDGETMUT
impl SizedBox {
    /// Give this container a child widget.
    ///
    /// If this container already has a child, it will be overwritten.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        if let Some(child) = this.widget.child.take() {
            this.ctx.remove_child(child);
        }
        this.widget.child = Some(child.erased().to_pod());
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// Remove the child widget.
    ///
    /// (If this widget has no child, this method does nothing.)
    pub fn remove_child(this: &mut WidgetMut<'_, Self>) {
        if let Some(child) = this.widget.child.take() {
            this.ctx.remove_child(child);
        }
    }

    /// Set container's width.
    pub fn set_width(this: &mut WidgetMut<'_, Self>, width: f64) {
        this.widget.width = Some(width);
        this.ctx.request_layout();
    }

    /// Set container's height.
    pub fn set_height(this: &mut WidgetMut<'_, Self>, height: f64) {
        this.widget.height = Some(height);
        this.ctx.request_layout();
    }

    /// Unset container's width.
    pub fn unset_width(this: &mut WidgetMut<'_, Self>) {
        this.widget.width = None;
        this.ctx.request_layout();
    }

    /// Unset container's height.
    pub fn unset_height(this: &mut WidgetMut<'_, Self>) {
        this.widget.height = None;
        this.ctx.request_layout();
    }

    /// Get mutable reference to the child widget, if any.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> Option<WidgetMut<'t, dyn Widget>> {
        let child = this.widget.child.as_mut()?;
        Some(this.ctx.get_mut(child))
    }
}

// --- MARK: INTERNALS
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
}

// --- MARK: IMPL WIDGET
impl Widget for SizedBox {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let bc = self.child_constraints(bc);
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        let origin = Point::ORIGIN;
        let origin = border.place_down(origin);
        let origin = padding.place_down(origin);

        let mut size;
        match self.child.as_mut() {
            Some(child) => {
                size = ctx.run_layout(child, &bc);
                ctx.place_child(child, origin);
            }
            None => {
                size = (self.width.unwrap_or(0.0), self.height.unwrap_or(0.0)).into();
                size = bc.constrain(size);
            }
        };

        let (size, _) = padding.layout_up(size, 0.);
        let (size, _) = border.layout_up(size, 0.);

        // TODO - figure out paint insets
        // TODO - figure out baseline offset

        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }
        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let bg = props.get::<Background>();
        let border_width = props.get::<BorderWidth>();
        let border_color = props.get::<BorderColor>();
        let corner_radius = props.get::<CornerRadius>();

        let bg_rect = border_width.bg_rect(ctx.size(), corner_radius);
        let border_rect = border_width.border_rect(ctx.size(), corner_radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        if let Some(child) = &self.child {
            ChildrenIds::from_slice(&[child.id()])
        } else {
            ChildrenIds::from_slice(&[])
        }
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("SizedBox", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Properties;
    use crate::palette;
    use crate::properties::types::{AsUnit, Gradient, UnitPoint};
    use crate::testing::{TestHarness, assert_failing_render_snapshot, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::Label;

    // TODO - Add WidgetMut tests

    #[test]
    fn expand() {
        let expand = SizedBox::new(Label::new("hello!").with_auto_id()).expand();
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(400., 400.,));
    }

    #[test]
    fn no_width() {
        let expand = SizedBox::new(Label::new("hello!").with_auto_id()).height(Length::px(200.));
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(0., 200.,));
        assert_eq!(child_bc.max(), Size::new(400., 200.,));
    }

    #[test]
    fn empty_box() {
        let mut box_props = Properties::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::empty()
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_empty_box");
    }

    #[test]
    fn label_box_no_size() {
        let mut box_props = Properties::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::new(Label::new("hello").with_auto_id()).with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_no_size");
    }

    #[test]
    fn label_box_with_size() {
        let mut box_props = Properties::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_size");
    }

    #[test]
    fn label_box_with_padding() {
        let mut box_props = Properties::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));
        box_props.insert(Padding::from_vh(15., 10.));

        let widget = SizedBox::new(Label::new("hello").with_auto_id()).with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_padding");
    }

    #[test]
    fn label_box_with_solid_background() {
        let mut box_props = Properties::new();
        box_props.insert(Background::Color(palette::css::PLUM));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_solid_background");
    }

    #[test]
    fn empty_box_with_gradient_background() {
        let mut box_props = Properties::new();

        let gradient = Gradient::new_linear(2.0).with_stops([
            palette::css::WHITE,
            palette::css::BLACK,
            palette::css::RED,
            palette::css::GREEN,
            palette::css::WHITE,
        ]);
        box_props.insert(Background::Gradient(gradient));
        box_props.insert(BorderColor::new(palette::css::LIGHT_SKY_BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(10.0));

        let widget = SizedBox::empty()
            .width(Length::px(20.))
            .height(Length::px(20.))
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_empty_box_with_gradient_background");
    }

    #[test]
    fn radial_gradient_background() {
        let mut box_props = Properties::new();

        let gradient = Gradient::new_radial(UnitPoint::CENTER).with_stops([
            palette::css::WHITE,
            palette::css::BLACK,
            palette::css::RED,
            palette::css::GREEN,
            palette::css::WHITE,
        ]);
        box_props.insert(Background::Gradient(gradient));
        box_props.insert(BorderColor::new(palette::css::LIGHT_SKY_BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(10.0));

        let widget = SizedBox::empty()
            .width(Length::px(20.))
            .height(Length::px(20.))
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_radial_gradient_background");
    }

    #[test]
    fn sweep_gradient_background() {
        let mut box_props = Properties::new();

        let gradient = Gradient::new_full_sweep(UnitPoint::CENTER, 0.).with_stops([
            palette::css::WHITE,
            palette::css::BLACK,
            palette::css::RED,
            palette::css::GREEN,
            palette::css::WHITE,
        ]);
        box_props.insert(Background::Gradient(gradient));
        box_props.insert(BorderColor::new(palette::css::LIGHT_SKY_BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(10.0));

        let widget = SizedBox::empty()
            .width(Length::px(20.))
            .height(Length::px(20.))
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_sweep_gradient_background");
    }

    #[test]
    fn label_box_with_padding_and_background() {
        let mut box_props = Properties::new();
        box_props.insert(Background::Color(palette::css::PLUM));
        box_props.insert(BorderColor::new(palette::css::LIGHT_SKY_BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(Padding::all(25.));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_background_and_padding");
    }

    // TODO - add screenshot tests for different brush types

    // --- MARK: INVALID SCREENSHOT TESTS

    #[test]
    fn invalid_screenshot() {
        // Copy-pasted from empty_box
        let mut box_props = Properties::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        // This is the difference
        box_props.insert(BorderWidth::all(5.2));

        let widget = SizedBox::empty()
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_failing_render_snapshot!(harness, "sized_box_empty_box");
    }
}
