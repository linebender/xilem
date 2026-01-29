// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::sync::Arc;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    Properties, PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, Update, UpdateCtx,
    Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::{
    Background, BorderColor, BorderWidth, ContentColor, CornerRadius, DisabledBackground,
    DisabledContentColor, Padding,
};
use crate::theme::{DISABLED_TEXT_COLOR, TEXT_COLOR};
use crate::util::{fill, stroke};
use crate::widgets::Label;

/// A non-interactive badge (pill) widget that hosts a single child.
///
/// Badges are typically used for short labels like "New", "Beta", or "99+".
///
#[doc = concat!(
    "![Badge with text](",
    include_doc_path!("screenshots/badge_with_text.png"),
    ")",
)]
///
/// This widget is non-interactive (it emits no actions). It uses the theme's per-widget
/// defaults for [`Background`], [`DisabledBackground`], [`Padding`], [`CornerRadius`], and
/// border properties.
///
/// # Examples
/// ```
/// use masonry::core::Widget;
/// use masonry::widgets::{Badge, Label};
///
/// let badge = Badge::new(Label::new("New").with_auto_id());
/// ```
pub struct Badge {
    child: WidgetPod<dyn Widget>,
}

/// How a numeric badge count is formatted when it exceeds a maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeCountOverflow {
    /// Always show the exact count value.
    Exact,
    /// If the count exceeds `max`, show `max` or `max+` depending on `show_plus`.
    Cap {
        /// The maximum value to display.
        max: u32,
        /// Whether to append a `+` when `count > max`.
        show_plus: bool,
    },
}

impl Default for BadgeCountOverflow {
    fn default() -> Self {
        Self::Cap {
            max: 99,
            show_plus: true,
        }
    }
}

// --- MARK: BUILDERS
impl Badge {
    /// Creates a new badge with the provided child.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
        }
    }

    /// Creates a new badge containing a styled text label.
    ///
    /// This is a convenience helper for the common "pill with text" use-case.
    ///
    /// # Examples
    /// ```
    /// use masonry::core::Widget;
    /// use masonry::widgets::Badge;
    ///
    /// let badge = Badge::with_text("New");
    /// ```
    pub fn with_text(text: impl Into<Arc<str>>) -> Self {
        use crate::parley::style::FontWeight;

        let label = Label::new(text)
            .with_style(StyleProperty::FontSize(12.0))
            .with_style(StyleProperty::FontWeight(FontWeight::BOLD))
            .with_props(
                Properties::new()
                    .with(ContentColor::new(TEXT_COLOR))
                    .with(DisabledContentColor(ContentColor::new(DISABLED_TEXT_COLOR))),
            );

        Self::new(label)
    }

    /// Creates a badge displaying a numeric count using the default overflow behavior.
    ///
    /// For a conditional badge that is hidden at `0`, see [`Badge::count_nonzero`].
    pub fn count(count: u32) -> Self {
        Self::count_with_overflow(count, BadgeCountOverflow::default())
    }

    /// Creates a badge displaying a numeric count, with explicit overflow formatting.
    pub fn count_with_overflow(count: u32, overflow: BadgeCountOverflow) -> Self {
        let text: Arc<str> = match overflow {
            BadgeCountOverflow::Exact => Arc::from(count.to_string().into_boxed_str()),
            BadgeCountOverflow::Cap { max, show_plus } => {
                if count > max {
                    if show_plus {
                        Arc::from(format!("{max}+").into_boxed_str())
                    } else {
                        Arc::from(max.to_string().into_boxed_str())
                    }
                } else {
                    Arc::from(count.to_string().into_boxed_str())
                }
            }
        };
        Self::with_text(text)
    }

    /// Creates a numeric badge only when `count` is non-zero.
    pub fn count_nonzero(count: u32) -> Option<Self> {
        (count != 0).then(|| Self::count(count))
    }
}

// --- MARK: WIDGETMUT
impl Badge {
    /// Replaces the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        this.ctx.remove_child(std::mem::replace(
            &mut this.widget.child,
            child.erased().to_pod(),
        ));
    }

    /// Returns a mutable reference to the child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl HasProperty<Background> for Badge {}
impl HasProperty<DisabledBackground> for Badge {}
impl HasProperty<BorderColor> for Badge {}
impl HasProperty<BorderWidth> for Badge {}
impl HasProperty<CornerRadius> for Badge {}

// --- MARK: IMPL WIDGET
impl Widget for Badge {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        DisabledBackground::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::DisabledChanged(_)) {
            ctx.request_paint_only();
        }
    }

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

        let cross = axis.cross();
        let cross_space = cross_length.map(|cross_length| {
            let cross_border_length = border.length(cross).dp(scale);
            let cross_padding_length = padding.length(cross).dp(scale);
            (cross_length - cross_border_length - cross_padding_length).max(0.)
        });

        let auto_length = len_req.reduce(border_length + padding_length).into();
        let context_size = LayoutSize::maybe(cross, cross_space);

        let child_length = ctx.compute_length(
            &mut self.child,
            auto_length,
            context_size,
            axis,
            cross_space,
        );

        child_length + border_length + padding_length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(space), space.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        let child_baseline = ctx.child_baseline_offset(&self.child);
        let child_baseline = border.baseline_up(child_baseline, scale);
        let child_baseline = padding.baseline_up(child_baseline, scale);
        let child_bottom = child_origin.y + child_size.height;
        let bottom_gap = size.height - child_bottom;
        ctx.set_baseline_offset(child_baseline + bottom_gap);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let border_color = props.get::<BorderColor>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);

        if border_width.width > 0.0 {
            stroke(scene, &border_rect, border_color.color, border_width.width);
        }
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
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Badge", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use masonry_testing::TestHarnessParams;

    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Label;

    #[test]
    fn badge_is_non_interactive() {
        let widget = Badge::new(Label::new("New").with_auto_id()).with_auto_id();
        let window_size = Size::new(80.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        let badge_id = harness.root_id();

        harness.mouse_click_on(badge_id);
        assert!(harness.pop_action_erased().is_none());
        assert!(harness.focused_widget().is_none());
    }

    #[test]
    fn badge_with_text() {
        let widget = Badge::with_text("New").with_auto_id();
        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::new(120.0, 60.0);
        params.root_padding = TestHarnessParams::ROOT_PADDING;
        let mut harness = TestHarness::create_with(test_property_set(), widget, params);

        assert_render_snapshot!(harness, "badge_with_text");
    }

    #[test]
    fn badge_count_overflow() {
        let widget = Badge::count(120).with_auto_id();
        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::new(120.0, 60.0);
        params.root_padding = TestHarnessParams::ROOT_PADDING;
        let mut harness = TestHarness::create_with(test_property_set(), widget, params);

        assert_render_snapshot!(harness, "badge_count_overflow");
    }
}
