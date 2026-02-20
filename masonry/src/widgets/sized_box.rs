// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Size};
use crate::layout::{LayoutSize, LenReq, Length};
use crate::properties::{BorderWidth, Padding};

/// A widget with bi-directional size enforcement.
///
/// It can either have an explicit size or it will adopt the size of its child.
///
/// ## Explicit size
///
/// There are two ways to define a size for `SizedBox`, in order of priority:
/// 1. [`Dimensions`] properties work as usual and take precedence over anything else.
/// 2. There are methods to configure the inner fields for width and height.
///
/// ## Adopted size
///
/// If there is no explicit size and the parent widget chooses to measure `SizedBox`,
/// then the `SizedBox::measure` method will forward its child's `measure` result.
/// This does not guarantee anything, but it usually means the parent will choose
/// the `SizedBox` child's measurement result as the size for `SizedBox`.
/// Set [`Dimensions::MAX`] for `SizedBox` to ensure there is no explicit size from props either.
///
/// ## Child's size
///
/// Whatever content-box size `SizedBox` ends up getting from its parent for layout,
/// `SizedBox` will force its child to use that same size for its border-box.
///
/// ## No child
///
/// The childless case works exactly as if there was a zero sized child.
/// The main impact being that the adopted size will be zero.
///
/// ## Borders and Padding
///
/// The explicit size may be increased to ensure that the border and padding fit.
/// When adopting the child's size, that size will be expanded by the `SizedBox` border and padding.
/// The size forced on the child is shrunk by the `SizedBox` border and padding.
///
/// [`Dimensions`]: crate::properties::Dimensions
/// [`Dimensions::MAX`]: crate::properties::Dimensions::MAX
#[doc = concat!(
    "![Box with blue border, pink background and a child label](",
    include_doc_path!("screenshots/sized_box_label_box_with_padding.png"),
    ")",
)]
pub struct SizedBox {
    child: Option<WidgetPod<dyn Widget>>,
    width: Option<Length>,
    height: Option<Length>,
}

// --- MARK: BUILDERS
impl SizedBox {
    /// Creates container with child, and both width and height unset.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: Some(child.erased().to_pod()),
            width: None,
            height: None,
        }
    }

    /// Creates container without a child, and both width and height unset.
    ///
    /// In this state it will render no content but will still render its border and padding.
    #[doc(alias = "null")]
    pub fn empty() -> Self {
        Self {
            child: None,
            width: None,
            height: None,
        }
    }

    /// Returns the container with `width`.
    pub fn width(mut self, width: Length) -> Self {
        self.width = Some(width);
        self
    }

    /// Returns the container with `height`.
    pub fn height(mut self, height: Length) -> Self {
        self.height = Some(height);
        self
    }

    /// Returns the container with `width` and `height`.
    pub fn size(mut self, width: Length, height: Length) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Returns the container with `width`.
    ///
    /// `None` means that the width will be adopted from the child.
    pub fn raw_width(mut self, width: Option<Length>) -> Self {
        self.width = width;
        self
    }

    /// Returns the container with `height`.
    ///
    /// `None` means that the height will be adopted from the child.
    pub fn raw_height(mut self, height: Option<Length>) -> Self {
        self.height = height;
        self
    }
}

// --- MARK: METHODS
impl SizedBox {
    /// Returns the length of the given `axis`.
    pub const fn length(&self, axis: Axis) -> Option<Length> {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}

// --- MARK: WIDGETMUT
impl SizedBox {
    /// Replaces the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        if let Some(child) = this.widget.child.take() {
            this.ctx.remove_child(child);
        }
        this.widget.child = Some(child.erased().to_pod());
        this.ctx.children_changed();
    }

    /// Removes the child widget.
    ///
    /// (If this widget has no child, this method does nothing.)
    pub fn remove_child(this: &mut WidgetMut<'_, Self>) {
        if let Some(child) = this.widget.child.take() {
            this.ctx.remove_child(child);
        }
    }

    /// Sets container's width.
    pub fn set_width(this: &mut WidgetMut<'_, Self>, width: Length) {
        this.widget.width = Some(width);
        this.ctx.request_layout();
    }

    /// Sets container's height.
    pub fn set_height(this: &mut WidgetMut<'_, Self>, height: Length) {
        this.widget.height = Some(height);
        this.ctx.request_layout();
    }

    /// Sets container's width and height.
    pub fn set_size(this: &mut WidgetMut<'_, Self>, width: Length, height: Length) {
        this.widget.width = Some(width);
        this.widget.height = Some(height);
        this.ctx.request_layout();
    }

    /// Unsets container's width.
    pub fn unset_width(this: &mut WidgetMut<'_, Self>) {
        this.widget.width = None;
        this.ctx.request_layout();
    }

    /// Unsets container's height.
    pub fn unset_height(this: &mut WidgetMut<'_, Self>) {
        this.widget.height = None;
        this.ctx.request_layout();
    }

    /// Sets the container's `width` directly.
    ///
    /// `None` means that the width will be adopted from the child.
    pub fn set_raw_width(this: &mut WidgetMut<'_, Self>, width: Option<Length>) {
        this.widget.width = width;
        this.ctx.request_layout();
    }

    /// Sets the container's `height` directly.
    ///
    /// `None` means that the height will be adopted from the child.
    pub fn set_raw_height(this: &mut WidgetMut<'_, Self>, height: Option<Length>) {
        this.widget.height = height;
        this.ctx.request_layout();
    }

    /// Returns mutable reference to the child widget, if any.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> Option<WidgetMut<'t, dyn Widget>> {
        let child = this.widget.child.as_mut()?;
        Some(this.ctx.get_mut(child))
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

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let border_length = border.length(axis).dp(scale);
        let padding_length = padding.length(axis).dp(scale);

        // First see if we have an explicitly defined length
        if let Some(length) = self.length(axis) {
            return (length.dp(scale) - border_length - padding_length).max(0.);
        }

        // Otherwise measure the child
        if let Some(child) = self.child.as_mut() {
            let cross = axis.cross();
            let cross_length = cross_length.or_else(|| {
                // Can't use self.length() due to borrow checker stupidity,
                // so we need to manually inline that method.
                let length = match cross {
                    Axis::Horizontal => self.width,
                    Axis::Vertical => self.height,
                };
                length.map(|length| {
                    let cross_border_length = border.length(cross).dp(scale);
                    let cross_padding_length = padding.length(cross).dp(scale);
                    (length.dp(scale) - cross_border_length - cross_padding_length).max(0.)
                })
            });

            let auto_length = len_req.into();
            let context_size = LayoutSize::maybe(cross, cross_length);

            ctx.compute_length(child, auto_length, context_size, axis, cross_length)
        } else {
            0.
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let Some(child) = self.child.as_mut() else {
            // No child, so no layout work beyond resetting the baseline
            ctx.clear_baseline_offset();
            return;
        };

        ctx.run_layout(child, size);
        ctx.place_child(child, Point::ORIGIN);

        let child_baseline = ctx.child_baseline_offset(child);
        ctx.set_baseline_offset(child_baseline);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

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
    use crate::core::PropertySet;
    use crate::layout::{AsUnit, UnitPoint};
    use crate::palette;
    use crate::properties::types::Gradient;
    use crate::properties::{Background, BorderColor, CornerRadius};
    use crate::testing::{TestHarness, assert_failing_render_snapshot, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Label;

    // TODO - Add WidgetMut tests

    #[test]
    fn empty_box() {
        let mut box_props = PropertySet::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::empty()
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_empty_box");
    }

    #[test]
    fn label_box_no_size() {
        let mut box_props = PropertySet::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::new(Label::new("hello").with_auto_id()).with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_no_size");
    }

    #[test]
    fn label_box_with_size() {
        let mut box_props = PropertySet::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_size");
    }

    #[test]
    fn label_box_with_padding() {
        let mut box_props = PropertySet::new();
        box_props.insert(BorderColor::new(palette::css::BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(CornerRadius::all(5.0));
        box_props.insert(Padding::from_vh(15., 10.));

        let widget = SizedBox::new(Label::new("hello").with_auto_id()).with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_padding");
    }

    #[test]
    fn label_box_with_solid_background() {
        let mut box_props = PropertySet::new();
        box_props.insert(Background::Color(palette::css::PLUM));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_solid_background");
    }

    #[test]
    fn empty_box_with_gradient_background() {
        let mut box_props = PropertySet::new();

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
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_empty_box_with_gradient_background");
    }

    #[test]
    fn radial_gradient_background() {
        let mut box_props = PropertySet::new();

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
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_radial_gradient_background");
    }

    #[test]
    fn sweep_gradient_background() {
        let mut box_props = PropertySet::new();

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
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_sweep_gradient_background");
    }

    #[test]
    fn label_box_with_padding_and_background() {
        let mut box_props = PropertySet::new();
        box_props.insert(Background::Color(palette::css::PLUM));
        box_props.insert(BorderColor::new(palette::css::LIGHT_SKY_BLUE));
        box_props.insert(BorderWidth::all(5.0));
        box_props.insert(Padding::all(25.));

        let widget = SizedBox::new(Label::new("hello").with_auto_id())
            .width(20.px())
            .height(20.px())
            .with_props(box_props);

        let window_size = Size::new(100.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "sized_box_label_box_with_background_and_padding");
    }

    // --- MARK: INVALID SCREENSHOT TESTS

    #[test]
    fn invalid_screenshot() {
        // Copy-pasted from empty_box
        let mut box_props = PropertySet::new();
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
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_failing_render_snapshot!(harness, "sized_box_empty_box");
    }
}
