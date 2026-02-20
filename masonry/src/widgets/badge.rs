// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, PropertySet, RegisterCtx, StyleProperty, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::parley::style::FontWeight;
use crate::properties::{ContentColor, DisabledContentColor};
use crate::theme::{DISABLED_TEXT_COLOR, TEXT_COLOR};
use crate::widgets::Label;

/// A non-interactive badge (pill) widget that hosts a single child.
///
/// Badges are typically used for short labels like "New", "Beta", or "99+".
///
/// To overlay a badge on top of another widget (for example, an unread count on an "Inbox"
/// button), see [`Badged`](crate::widgets::Badged).
///
#[doc = concat!(
    "![Badge with text](",
    include_doc_path!("screenshots/badge_with_text.png"),
    ")",
)]
///
/// # Examples
/// ```
/// use masonry::core::Widget;
/// use masonry::widgets::{Badge, Label};
///
/// let badge = Badge::new(Label::new("New").with_auto_id());
/// ```
///
/// [`Background`]: crate::properties::Background
/// [`CornerRadius`]: crate::properties::CornerRadius
/// [`DisabledBackground`]: crate::properties::DisabledBackground
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
        let label = Label::new(text)
            .with_style(StyleProperty::FontSize(12.0))
            .with_style(StyleProperty::FontWeight(FontWeight::BOLD))
            .with_props(
                PropertySet::new()
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

// --- MARK: IMPL WIDGET
impl Widget for Badge {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::DisabledChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        ctx.compute_length(
            &mut self.child,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        let child_baseline = ctx.child_baseline_offset(&self.child);
        let child_bottom = child_origin.y + child_size.height;
        let bottom_gap = size.height - child_bottom;
        ctx.set_baseline_offset(child_baseline + bottom_gap);
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

    use crate::testing::{TestHarness, TestHarnessParams, assert_render_snapshot};
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
