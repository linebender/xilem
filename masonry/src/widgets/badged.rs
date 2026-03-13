// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Size, Vec2};
use crate::layout::{LayoutSize, LenReq, SizeDef};

/// Where a badge is placed relative to the content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgePlacement {
    /// Place the badge at the content's top-left corner.
    TopLeft,
    /// Place the badge at the content's top-right corner.
    TopRight,
    /// Place the badge at the content's bottom-left corner.
    BottomLeft,
    /// Place the badge at the content's bottom-right corner.
    BottomRight,
}

impl BadgePlacement {
    fn resolve(self, content_size: Size) -> Point {
        match self {
            Self::TopLeft => Point::new(0.0, 0.0),
            Self::TopRight => Point::new(content_size.width, 0.0),
            Self::BottomLeft => Point::new(0.0, content_size.height),
            Self::BottomRight => Point::new(content_size.width, content_size.height),
        }
    }
}

/// A widget that overlays a "badge" widget on top of some content.
///
/// This is useful for adding count/status badges to existing widgets, like a button
/// ("Inbox" + "3") or an avatar with an "online" dot.
///
/// The overlaid badge is often a [`Badge`](crate::widgets::Badge).
///
#[doc = concat!(
    "![Badged button](",
    include_doc_path!("screenshots/badged_button.png"),
    ")",
)]
///
/// The `Badged` widget's size is determined solely by its content; the badge does not
/// affect layout and may overflow the content bounds.
pub struct Badged {
    content: WidgetPod<dyn Widget>,
    badge: Option<WidgetPod<dyn Widget>>,
    placement: BadgePlacement,
    offset: Vec2,
}

// --- MARK: BUILDERS
impl Badged {
    /// Creates a new `Badged` widget.
    ///
    /// By default, the badge is placed at the content's top-right corner, with the badge's center
    /// anchored to that corner (so it overlaps the content by about half).
    pub fn new(
        content: NewWidget<impl Widget + ?Sized>,
        badge: NewWidget<impl Widget + ?Sized>,
    ) -> Self {
        Self {
            content: content.erased().to_pod(),
            badge: Some(badge.erased().to_pod()),
            placement: BadgePlacement::TopRight,
            offset: Vec2::ZERO,
        }
    }

    /// Creates a new `Badged` widget that may omit the badge entirely.
    ///
    /// This is useful for conditional badges, e.g. hiding an unread-count badge when the count is 0.
    pub fn new_optional(
        content: NewWidget<impl Widget + ?Sized>,
        badge: Option<NewWidget<dyn Widget>>,
    ) -> Self {
        Self {
            content: content.erased().to_pod(),
            badge: badge.map(|b| b.to_pod()),
            placement: BadgePlacement::TopRight,
            offset: Vec2::ZERO,
        }
    }

    /// Returns whether a badge is currently present.
    pub fn has_badge(&self) -> bool {
        self.badge.is_some()
    }

    /// Sets where the badge is placed relative to the content.
    pub fn with_badge_placement(mut self, placement: BadgePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets an additional offset applied after placing the badge.
    ///
    /// This can be used to nudge the badge further outside or inside the content bounds.
    pub fn with_badge_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}

// --- MARK: WIDGETMUT
impl Badged {
    /// Replaces the content widget with a new one.
    pub fn set_content(this: &mut WidgetMut<'_, Self>, content: NewWidget<impl Widget + ?Sized>) {
        this.ctx.remove_child(std::mem::replace(
            &mut this.widget.content,
            content.erased().to_pod(),
        ));
    }

    /// Replaces the badge widget with a new one.
    pub fn set_badge(this: &mut WidgetMut<'_, Self>, badge: NewWidget<impl Widget + ?Sized>) {
        if let Some(old) = this.widget.badge.replace(badge.erased().to_pod()) {
            this.ctx.remove_child(old);
        } else {
            this.ctx.children_changed();
        }
    }

    /// Sets where the badge is placed relative to the content.
    pub fn set_badge_placement(this: &mut WidgetMut<'_, Self>, placement: BadgePlacement) {
        if this.widget.placement != placement {
            this.widget.placement = placement;
            this.ctx.request_layout();
        }
    }

    /// Sets an additional offset applied after placing the badge.
    pub fn set_badge_offset(this: &mut WidgetMut<'_, Self>, offset: Vec2) {
        if this.widget.offset != offset {
            this.widget.offset = offset;
            this.ctx.request_layout();
        }
    }

    /// Removes the badge entirely.
    pub fn clear_badge(this: &mut WidgetMut<'_, Self>) {
        if let Some(old) = this.widget.badge.take() {
            this.ctx.remove_child(old);
        }
    }

    /// Returns a mutable reference to the content widget.
    pub fn content_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.content)
    }

    /// Returns a mutable reference to the badge widget.
    pub fn badge_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> Option<WidgetMut<'t, dyn Widget>> {
        this.widget
            .badge
            .as_mut()
            .map(|badge| this.ctx.get_mut(badge))
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Badged {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.content);
        if let Some(badge) = &mut self.badge {
            ctx.register_child(badge);
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
            &mut self.content,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let content_size = ctx.compute_size(&mut self.content, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.content, content_size);
        ctx.place_child(&mut self.content, Point::ORIGIN);
        ctx.derive_baselines(&self.content);

        let Some(badge) = &mut self.badge else {
            return;
        };

        let badge_size = ctx.compute_size(badge, SizeDef::MAX, size.into());
        ctx.run_layout(badge, badge_size);

        let content_anchor = self.placement.resolve(content_size);

        let badge_origin = content_anchor + self.offset
            - Vec2::new(badge_size.width * 0.5, badge_size.height * 0.5);
        ctx.place_child(badge, badge_origin);
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
        match &self.badge {
            Some(badge) => ChildrenIds::from_slice(&[self.content.id(), badge.id()]),
            None => ChildrenIds::from_slice(&[self.content.id()]),
        }
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Badged", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::{TestHarness, TestHarnessParams, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Align, Badge, Button};

    #[test]
    fn badged_button() {
        let widget = Align::centered(
            Badged::new(
                Button::with_text("Inbox").with_auto_id(),
                Badge::with_text("3").with_auto_id(),
            )
            .with_auto_id(),
        )
        .with_auto_id();

        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::new(240.0, 120.0);
        params.root_padding = TestHarnessParams::ROOT_PADDING;
        let mut harness = TestHarness::create_with(test_property_set(), widget, params);

        assert_render_snapshot!(harness, "badged_button");
    }

    #[test]
    fn badged_button_optional_badge() {
        let widget = Align::centered(
            Badged::new_optional(Button::with_text("Inbox").with_auto_id(), None).with_auto_id(),
        )
        .with_auto_id();

        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::new(240.0, 120.0);
        params.root_padding = TestHarnessParams::ROOT_PADDING;
        let mut harness = TestHarness::create_with(test_property_set(), widget, params);

        assert_render_snapshot!(harness, "badged_button_no_badge");
    }
}
