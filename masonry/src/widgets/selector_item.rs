// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A button widget.

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::NoAction;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::core::{HasProperty, MeasureCtx};
use crate::kurbo::{Affine, Axis, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, FocusedBorderColor, HoveredBorderColor, Padding,
};
use crate::util::{fill, stroke};
use crate::widgets::Label;

/// An option in a [`SelectorMenu`](crate::layers::SelectorMenu).
pub struct SelectorItem {
    child: WidgetPod<Label>,
}

// --- MARK: BUILDERS
impl SelectorItem {
    /// Creates new selector item with the given text.
    pub fn new(text: String) -> Self {
        Self {
            child: WidgetPod::new(Label::new(text)),
        }
    }
}

// --- MARK: WIDGETMUT
impl SelectorItem {
    /// Returns a mutable reference to the child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl HasProperty<DisabledBackground> for SelectorItem {}
impl HasProperty<ActiveBackground> for SelectorItem {}
impl HasProperty<Background> for SelectorItem {}
impl HasProperty<FocusedBorderColor> for SelectorItem {}
impl HasProperty<HoveredBorderColor> for SelectorItem {}
impl HasProperty<BorderColor> for SelectorItem {}
impl HasProperty<BorderWidth> for SelectorItem {}
impl HasProperty<CornerRadius> for SelectorItem {}
impl HasProperty<Padding> for SelectorItem {}
impl HasProperty<BoxShadow> for SelectorItem {}

// --- MARK: IMPL WIDGET
impl Widget for SelectorItem {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_)
            | Update::ActiveChanged(_)
            | Update::FocusChanged(_)
            | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        DisabledBackground::prop_changed(ctx, property_type);
        ActiveBackground::prop_changed(ctx, property_type);
        Background::prop_changed(ctx, property_type);
        FocusedBorderColor::prop_changed(ctx, property_type);
        HoveredBorderColor::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        BoxShadow::prop_changed(ctx, property_type);
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
        let shadow = props.get::<BoxShadow>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(space), space.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        let baseline = ctx.child_baseline_offset(&self.child);
        let baseline = border.baseline_up(baseline, scale);
        let baseline = padding.baseline_up(baseline, scale);
        ctx.set_baseline_offset(baseline);

        if shadow.is_visible() {
            ctx.set_paint_insets(shadow.get_insets());
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let is_focused = ctx.is_focus_target();
        let is_pressed = ctx.is_active();
        let is_hovered = ctx.is_hovered();
        let size = ctx.size();

        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else if is_pressed {
            &props.get::<ActiveBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        let border_color = if is_focused {
            &props.get::<FocusedBorderColor>().0
        } else if is_hovered {
            &props.get::<HoveredBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let border_radius = props.get::<CornerRadius>();
        let shadow = props.get::<BoxShadow>();

        let shadow_rect = shadow.shadow_rect(size, border_radius);

        shadow.paint(scene, Affine::IDENTITY, shadow_rect);
    }

    fn accessibility_role(&self) -> Role {
        Role::MenuItem
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("SelectorItem", id = id.trace())
    }
}
