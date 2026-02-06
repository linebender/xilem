// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role, Toggled};
use include_doc_path::include_doc_path;
use tracing::{Span, trace, trace_span};
use vello::Scene;

use crate::core::keyboard::Key;
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, HasProperty, LayoutCtx, MeasureCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut,
};
use crate::kurbo::{Axis, Circle, Point, Rect, Size};
use crate::layout::LenReq;
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, CornerRadius, DisabledBackground,
    FocusedBorderColor, HoveredBorderColor, ThumbColor, ThumbRadius, ToggledBackground,
    TrackThickness,
};
use crate::util::{fill, stroke};

/// A switch switch that can be turned on or off.
///
#[doc = concat!(
    "![Switch in on state](",
    include_doc_path!("screenshots/switch_on_initial.png"),
    ")",
)]
///
/// This is a boolean control similar to a checkbox, but with a sliding switch appearance.
/// The switch displays a track with a circular thumb that sits on the left when off
/// and on the right when on.
///
/// Emits [`SwitchToggled`] when the user activates it.
/// Note that the on state does not automatically switch, and so one of
/// the responses to a `SwitchToggled` is to call [`Switch::set_on`]
/// on the originating widget.
///
/// This allows higher-level components to choose how the switch responds,
/// and ensure that its value is based on their correct source of truth.
pub struct Switch {
    on: bool,
}

// --- MARK: BUILDERS
impl Switch {
    /// Creates a new `Switch` with the given initial state.
    pub fn new(on: bool) -> Self {
        Self { on }
    }

    /// Returns whether the switch is currently on.
    pub fn is_on(&self) -> bool {
        self.on
    }
}

// --- MARK: WIDGETMUT
impl Switch {
    /// Sets the switch state.
    pub fn set_on(this: &mut WidgetMut<'_, Self>, on: bool) {
        if this.widget.on != on {
            this.widget.on = on;
            // On state impacts appearance and accessibility node
            this.ctx.request_render();
        }
    }
}

// --- MARK: HELPERS
impl Switch {
    /// Calculates the track dimensions based on properties.
    ///
    /// Returns `(track_width, track_height)`.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "PropertiesRef is given to Widget as a ref"
    )]
    fn track_dimensions(props: &PropertiesRef<'_>, scale: f64) -> (f64, f64) {
        let track_thickness = props.get::<TrackThickness>().0 * scale;
        let thumb_radius = props.get::<ThumbRadius>().0 * scale;

        // The track height is the larger of track_thickness or thumb diameter
        let track_height = track_thickness.max(thumb_radius * 2.0);
        // The track width is approximately 2x the height (pill shape)
        let track_width = track_height * 2.0;

        (track_width, track_height)
    }
}

impl HasProperty<ToggledBackground> for Switch {}
impl HasProperty<ThumbRadius> for Switch {}
impl HasProperty<ThumbColor> for Switch {}
impl HasProperty<TrackThickness> for Switch {}

/// The action type emitted by [`Switch`] when it is activated.
///
/// The field is the target switch state (i.e. true is "this switch would like to become on").
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SwitchToggled(pub bool);

// --- MARK: IMPL WIDGET
impl Widget for Switch {
    type Action = SwitchToggled;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                ctx.request_focus();
                ctx.capture_pointer();
                trace!("Switch {:?} pressed", ctx.widget_id());
            }
            PointerEvent::Up { .. } => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<Self::Action>(SwitchToggled(!self.on));
                    trace!("Switch {:?} released", ctx.widget_id());
                }
            }
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(event) if event.state.is_up() => {
                // Space toggles the switch (per ARIA guidelines, Enter should submit forms)
                if matches!(&event.key, Key::Character(c) if c == " ") {
                    ctx.submit_action::<Self::Action>(SwitchToggled(!self.on));
                    ctx.set_handled();
                }
            }
            _ => (),
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            accesskit::Action::Click => {
                ctx.submit_action::<Self::Action>(SwitchToggled(!self.on));
            }
            _ => {}
        }
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

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        ToggledBackground::prop_changed(ctx, property_type);
        ThumbRadius::prop_changed(ctx, property_type);
        ThumbColor::prop_changed(ctx, property_type);
        TrackThickness::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let (track_width, track_height) = Self::track_dimensions(props, scale);

        match axis {
            Axis::Horizontal => track_width,
            Axis::Vertical => track_height,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn pre_paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _scene: &mut Scene,
    ) {
        // TODO: Make Switch painting work with generic shadow/background/border
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let is_focused = ctx.is_focus_target();
        let is_pressed = ctx.is_active();
        let is_hovered = ctx.is_hovered();
        let is_disabled = ctx.is_disabled();

        let size = ctx.border_box_size();

        let (track_width, track_height) = Self::track_dimensions(props, scale);
        let thumb_radius = props.get::<ThumbRadius>().0 * scale;
        let border_width = props.get::<BorderWidth>().width * scale;
        let corner_radius = props.get::<CornerRadius>().radius * scale;
        let thumb_color = props.get::<ThumbColor>().0;

        // Center the track within the available space
        let track_x = (size.width - track_width) / 2.0;
        let track_y = (size.height - track_height) / 2.0;
        let track_rect = Rect::new(
            track_x,
            track_y,
            track_x + track_width,
            track_y + track_height,
        ) - ctx.border_box_translation();

        // Determine track background color
        let track_bg = if is_disabled && let Some(db) = props.get_defined::<DisabledBackground>() {
            &db.0
        } else if is_pressed && let Some(ab) = props.get_defined::<ActiveBackground>() {
            &ab.0
        } else if self.on
            && let Some(tb) = props.get_defined::<ToggledBackground>()
        {
            &tb.0
        } else {
            props.get::<Background>()
        };

        // Paint track background
        let track_corner_radius = corner_radius.min(track_height / 2.0);
        let track_rounded = track_rect.to_rounded_rect(track_corner_radius);
        let brush = track_bg.get_peniko_brush_for_rect(track_rect);
        fill(scene, &track_rounded, &brush);

        // Determine border color
        let border_color = if is_focused && let Some(fb) = props.get_defined::<FocusedBorderColor>()
        {
            &fb.0
        } else if is_hovered && let Some(hb) = props.get_defined::<HoveredBorderColor>() {
            &hb.0
        } else {
            props.get::<BorderColor>()
        };

        // Paint track border
        if border_width > 0.0 {
            stroke(scene, &track_rounded, border_color.color, border_width);
        }

        // Calculate thumb position (centered vertically, left/right based on state)
        let thumb_y = size.height / 2.0 - ctx.border_box_translation().y;
        let thumb_x = if self.on {
            // Thumb on the right
            track_rect.x1 - thumb_radius - border_width / 2.0
        } else {
            // Thumb on the left
            track_rect.x0 + thumb_radius + border_width / 2.0
        };

        // Paint thumb
        let thumb_circle = Circle::new(Point::new(thumb_x, thumb_y), thumb_radius);
        let thumb_brush = if is_disabled {
            thumb_color.with_alpha(0.5)
        } else {
            thumb_color
        };
        fill(scene, &thumb_circle, thumb_brush);
    }

    fn accessibility_role(&self) -> Role {
        Role::Switch
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
        if self.on {
            node.set_toggled(Toggled::True);
        } else {
            node.set_toggled(Toggled::False);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Switch", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        if self.on {
            Some("[ON]".to_string())
        } else {
            Some("[OFF]".to_string())
        }
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TextEvent;
    use crate::properties::types::{CrossAxisAlignment, MainAxisAlignment};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Flex;

    // --- MARK: NON-RENDERING BEHAVIOR TESTS

    #[test]
    fn click_emits_action_and_focuses() {
        let widget = Switch::new(false).with_auto_id();
        let window_size = Size::new(60.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        let switch_id = harness.root_id();

        // Initially not focused, and no actions
        assert!(harness.focused_widget().is_none());
        // Initially no actions
        assert!(harness.pop_action_erased().is_none());

        // Click on switch (off -> wants to be on)
        harness.mouse_click_on(switch_id);
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(switch_id));
        assert_eq!(
            harness.pop_action::<SwitchToggled>(),
            Some((SwitchToggled(true), switch_id))
        );

        // Update state to on
        harness.edit_root_widget(|mut switch| Switch::set_on(&mut switch, true));

        // Click again (on -> wants to be off)
        harness.mouse_click_on(switch_id);
        assert_eq!(
            harness.pop_action::<SwitchToggled>(),
            Some((SwitchToggled(false), switch_id))
        );
    }

    #[test]
    fn space_emits_action_when_focused() {
        let widget = Switch::new(false).with_auto_id();
        let window_size = Size::new(60.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        let switch_id = harness.root_id();

        // Focus via tab
        harness.focus_on(None);
        harness.press_tab_key(false);
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(switch_id));

        // Space key should switch (off -> wants on)
        harness.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
        harness.process_text_event(TextEvent::key_up(Key::Character(" ".into())));
        assert_eq!(
            harness.pop_action::<SwitchToggled>(),
            Some((SwitchToggled(true), switch_id))
        );

        // Update state
        harness.edit_root_widget(|mut switch| Switch::set_on(&mut switch, true));

        // Space again (on -> wants off)
        harness.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
        harness.process_text_event(TextEvent::key_up(Key::Character(" ".into())));
        assert_eq!(
            harness.pop_action::<SwitchToggled>(),
            Some((SwitchToggled(false), switch_id))
        );
    }

    #[test]
    fn measure_dimensions() {
        // Test that the switch measures to expected dimensions based on theme properties.
        // Theme defaults: ThumbRadius(8.0), TrackThickness(20.0), BorderWidth(1.0)
        // Expected: track_height = max(20, 8*2) = 20, track_width = 20*2 = 40
        // With borders: width = 42, height = 22
        let switch = Switch::new(false).with_auto_id();
        let switch_id = switch.id();

        // Wrap in Flex with Start alignment so it doesn't stretch the switch
        let flex = Flex::row()
            .with_fixed(switch)
            .main_axis_alignment(MainAxisAlignment::Start)
            .cross_axis_alignment(CrossAxisAlignment::Start);

        // Give it much more space than needed
        let window_size = Size::new(200.0, 100.0);
        let harness =
            TestHarness::create_with_size(test_property_set(), flex.with_auto_id(), window_size);

        let size = harness
            .get_widget_with_id(switch_id)
            .ctx()
            .border_box_size();

        // Switch should maintain its intrinsic size, not fill available space
        assert_eq!(
            size.width, 42.0,
            "Switch width should be 2*track_height + 2*border"
        );
        assert_eq!(
            size.height, 22.0,
            "Switch height should be track_height + 2*border"
        );
    }

    // --- MARK: SNAPSHOT TESTS

    #[test]
    fn simple_switch() {
        let widget = Switch::new(false).with_auto_id();

        let window_size = Size::new(60.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        let switch_id = harness.root_id();

        assert_render_snapshot!(harness, "switch_off");

        assert!(harness.pop_action_erased().is_none());

        // Hover without clicking to show hovered state (no focus)
        harness.mouse_move_to(switch_id);
        assert_render_snapshot!(harness, "switch_off_hovered");

        // Now click to switch
        harness.mouse_click_on(switch_id);
        assert_eq!(
            harness.pop_action::<SwitchToggled>(),
            Some((SwitchToggled(true), switch_id))
        );

        harness.edit_root_widget(|mut switch| Switch::set_on(&mut switch, true));

        // After clicking, the switch is both focused and hovered
        assert_render_snapshot!(harness, "switch_on_focused_hovered");
    }

    #[test]
    fn focus_visual() {
        let widget = Switch::new(false).with_auto_id();

        let window_size = Size::new(60.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        let switch_id = harness.root_id();

        // Focus directly (not via click) to get focused-but-not-hovered state
        harness.focus_on(Some(switch_id));
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(switch_id));

        assert_render_snapshot!(harness, "switch_focused");
    }

    #[test]
    fn on_state() {
        let widget = Switch::new(true).with_auto_id();

        let window_size = Size::new(60.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "switch_on_initial");
    }
}
