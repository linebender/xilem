// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::{HasProperty, Layer, NoAction, PointerUpdate};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Point, Size};

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, FocusedBorderColor, HoveredBorderColor, Padding,
};
use crate::theme;
use crate::util::{fill, stroke};

/// A [`Layer`] representing a simple tooltip showing some content until the mouse moves.
pub struct Tooltip {
    child: WidgetPod<dyn Widget>,
    pointer_pos: Point,
}

/// How much the user can move their mouse before the tooltip disappears.
const TOOLTIP_TOLERANCE: f64 = 4.0;

// --- MARK: BUILDERS
impl Tooltip {
    /// Creates a new `Tooltip`.
    ///
    /// `pointer_pos` is the current position of the pointer that created this tooltip (usually the mouse).
    pub fn new(child: NewWidget<impl Widget + ?Sized>, pointer_pos: Point) -> Self {
        Self {
            child: child.erased().to_pod(),
            pointer_pos,
        }
    }
}

// --- MARK: WIDGETMUT
impl Tooltip {
    /// Replace the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        this.ctx.remove_child(std::mem::replace(
            &mut this.widget.child,
            child.erased().to_pod(),
        ));
    }

    /// Get a mutable reference to the child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl HasProperty<Background> for Tooltip {}
impl HasProperty<BorderColor> for Tooltip {}
impl HasProperty<BorderWidth> for Tooltip {}
impl HasProperty<CornerRadius> for Tooltip {}
impl HasProperty<Padding> for Tooltip {}
impl HasProperty<BoxShadow> for Tooltip {}

// --- MARK: IMPL WIDGET
impl Widget for Tooltip {
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
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        BoxShadow::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let shadow = props.get::<BoxShadow>();

        let initial_bc = bc;

        let bc = bc.loosen();
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        let label_size = ctx.run_layout(&mut self.child, &bc);
        let baseline = ctx.child_baseline_offset(&self.child);

        let size = label_size;
        let (size, baseline) = padding.layout_up(size, baseline);
        let (size, baseline) = border.layout_up(size, baseline);

        // TODO - Add MinimumSize property.
        // HACK: to make sure we look okay at default sizes when beside a text input,
        // we make sure we will have at least the same height as the default text input.
        let mut size = size;
        size.height = size.height.max(theme::BORDERED_WIDGET_HEIGHT);

        // TODO - Figure out how to handle cases where label size doesn't fit bc.
        let size = initial_bc.constrain(size);
        let label_offset = (size.to_vec2() - label_size.to_vec2()) / 2.0;
        ctx.place_child(&mut self.child, label_offset.to_point());

        // TODO - pos = (size - label_size) / 2

        if shadow.is_visible() {
            ctx.set_paint_insets(shadow.get_insets());
        }

        ctx.set_baseline_offset(baseline);
        size
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
        Role::Tooltip
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

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Tooltip", id = id.trace())
    }
}

// --- MARK: IMPL LAYER
impl Layer for Tooltip {
    fn capture_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let remove_tooltip = match event {
            PointerEvent::Down(_) | PointerEvent::Up(_) | PointerEvent::Leave(_) => true,

            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let current_pos = current.logical_point();
                let mouse_has_moved = current_pos.distance(self.pointer_pos) > TOOLTIP_TOLERANCE;
                mouse_has_moved
            }

            _ => false,
        };

        if remove_tooltip {
            ctx.remove_layer(ctx.widget_id());
        }
    }
}
