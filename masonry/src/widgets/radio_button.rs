// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A radio button widget.

use std::any::TypeId;

use accesskit::{Node, Role, Toggled};
use masonry_core::core::HasProperty;
use tracing::{Span, trace, trace_span};
use ui_events::keyboard::Key;
use vello::Scene;
use vello::kurbo::Rect;
use vello::kurbo::{Affine, Cap, Dashes, Join, Size, Stroke};

use crate::core::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, CheckmarkColor, CheckmarkStrokeWidth,
    CornerRadius, DisabledBackground, DisabledCheckmarkColor, HoveredBorderColor, Padding,
};
use crate::theme;
use crate::util::{fill, include_screenshot, stroke};
use crate::widgets::Label;

/// A radio button that can be toggled.
///
#[doc = include_screenshot!("radio_hello_checked.png", "Radio button with checked state.")]
///
/// Emits [`RadioToggled`] when it should toggle.
/// Note that the checked state does not automatically toggle, and so one of
/// the responses to a `RadioToggled` is to call [`RadioButton::set_checked`]
/// on the originating widget.
///
/// This allows higher-level components to choose how the radio responds,
/// and ensure that its value is based on their correct source of truth.
pub struct RadioButton {
    checked: bool,
    // FIXME - Remove label child, have this widget only be a box with a checkmark.
    label: WidgetPod<Label>,
}

impl RadioButton {
    /// Create a new `RadioButton` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Self {
        Self {
            checked,
            label: WidgetPod::new(Label::new(text)),
        }
    }

    /// Create a new `RadioButton` with the given label.
    pub fn from_label(checked: bool, label: NewWidget<Label>) -> Self {
        Self {
            checked,
            label: label.to_pod(),
        }
    }
}

// --- MARK: WIDGETMUT
impl RadioButton {
    /// Check or uncheck the box.
    pub fn set_checked(this: &mut WidgetMut<'_, Self>, checked: bool) {
        this.widget.checked = checked;
        // Checked state impacts appearance and accessibility node
        this.ctx.request_render();
    }

    /// Set the text.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: ArcStr) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    /// Get a mutable reference to the label.
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }
}

impl HasProperty<DisabledBackground> for RadioButton {}
impl HasProperty<ActiveBackground> for RadioButton {}
impl HasProperty<Background> for RadioButton {}
impl HasProperty<HoveredBorderColor> for RadioButton {}
impl HasProperty<BorderColor> for RadioButton {}
impl HasProperty<BorderWidth> for RadioButton {}
impl HasProperty<CornerRadius> for RadioButton {}
impl HasProperty<Padding> for RadioButton {}
impl HasProperty<CheckmarkStrokeWidth> for RadioButton {}
impl HasProperty<DisabledCheckmarkColor> for RadioButton {}
impl HasProperty<CheckmarkColor> for RadioButton {}

/// The action type emitted by [`RadioButton`] when it is activated.
///
/// The field is the target toggle state (i.e. true is "this radio would like to become checked").
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RadioToggled(pub bool);

// --- MARK: IMPL WIDGET
impl Widget for RadioButton {
    type Action = RadioToggled;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                ctx.capture_pointer();
                trace!("Radio {:?} pressed", ctx.widget_id());
            }
            PointerEvent::Up { .. } => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<Self::Action>(RadioToggled(!self.checked));
                    trace!("Radio {:?} released", ctx.widget_id());
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
                if matches!(&event.key, Key::Character(c) if c == " ") {
                    ctx.submit_action::<Self::Action>(RadioToggled(!self.checked));
                }
            }
            _ => (),
        }
    }

    fn accepts_focus(&self) -> bool {
        // Radio can be tab-focused...
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
                ctx.submit_action::<Self::Action>(RadioToggled(!self.checked));
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

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        DisabledBackground::prop_changed(ctx, property_type);
        ActiveBackground::prop_changed(ctx, property_type);
        Background::prop_changed(ctx, property_type);
        HoveredBorderColor::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        CheckmarkStrokeWidth::prop_changed(ctx, property_type);
        DisabledCheckmarkColor::prop_changed(ctx, property_type);
        CheckmarkColor::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let x_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING;
        let check_side = theme::BASIC_WIDGET_HEIGHT;

        let label_size = ctx.run_layout(&mut self.label, bc);
        ctx.place_child(&mut self.label, (check_side + x_padding, 0.0).into());

        let check_size = Size::new(check_side, check_side);
        let (check_size, _) = padding.layout_up(check_size, 0.);
        let (check_size, _) = border.layout_up(check_size, 0.);

        let desired_size = Size::new(
            check_size.width + x_padding + label_size.width,
            check_size.height.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline =
            ctx.child_baseline_offset(&self.label) + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let is_pressed = ctx.is_active();
        let is_hovered = ctx.is_hovered();

        let check_size = theme::BASIC_WIDGET_HEIGHT;
        let size = Size::new(check_size, check_size);

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

        let border_color = if is_hovered {
            &props.get::<HoveredBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };

        // Paint the radio box background and border
        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);

        // Paint the checkmark if checked
        if self.checked {
            let brush = if ctx.is_disabled() {
                &props.get::<DisabledCheckmarkColor>().0
            } else {
                props.get::<CheckmarkColor>()
            };

            // TODO: Create a prop for ellipse size. Default: 50%
            let filled_ellipse = border_width
                .bg_ellipse(size * 0.5)
                .with_center(bg_rect.center());
            fill(scene, &filled_ellipse, brush.color);
        }
        // Paint focus indicator around the entire widget (box + label)
        if ctx.is_focus_target() || is_hovered {
            let widget_size = ctx.size();

            let focus_rect = Rect::new(0.0, 0.0, widget_size.width, widget_size.height);

            let focus_rect = focus_rect.inflate(2.0, 2.0);

            let focus_color = theme::FOCUS_COLOR;
            let focus_width = 2.0;
            let focus_radius = 4.0;

            let focus_stroke = Stroke {
                width: focus_width,
                join: Join::Round,
                miter_limit: 10.0,
                start_cap: Cap::Round,
                end_cap: Cap::Round,
                dash_pattern: Dashes::default(),
                dash_offset: 0.0,
            };
            let focus_path = focus_rect.to_rounded_rect(focus_radius);
            scene.stroke(
                &focus_stroke,
                Affine::IDENTITY,
                focus_color,
                None,
                &focus_path,
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::RadioButton
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
        if self.checked {
            node.set_toggled(Toggled::True);
        } else {
            node.set_toggled(Toggled::False);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Radio", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        if self.checked {
            Some("[X]".to_string())
        } else {
            Some("[ ]".to_string())
        }
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Properties, StyleProperty};
    use crate::properties::ContentColor;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, default_property_set};
    use crate::widgets::Flex;

    #[test]
    fn simple_radio() {
        let widget = NewWidget::new(RadioButton::new(false, "Hello"));

        let window_size = Size::new(100.0, 40.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);
        let radio_id = harness.root_id();

        assert_render_snapshot!(harness, "radio_hello_unchecked");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(radio_id);
        assert_eq!(
            harness.pop_action::<RadioToggled>(),
            Some((RadioToggled(true), radio_id))
        );

        assert_render_snapshot!(harness, "radio_hello_hovered");

        harness.edit_root_widget(|mut radio| RadioButton::set_checked(&mut radio, true));

        assert_render_snapshot!(harness, "radio_hello_checked");

        harness.focus_on(None);
        harness.press_tab_key(false);
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(radio_id));

        harness.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
        harness.process_text_event(TextEvent::key_up(Key::Character(" ".into())));
        assert_eq!(
            harness.pop_action::<RadioToggled>(),
            Some((RadioToggled(false), radio_id))
        );
    }

    #[test]
    fn radio_focus_indicator() {
        use crate::properties::types::MainAxisAlignment;

        let radio = NewWidget::new(RadioButton::new(true, "Focus test"));
        let radio_id = radio.id();

        let root = NewWidget::new(
            Flex::row()
                .with_child(radio)
                .main_axis_alignment(MainAxisAlignment::Center),
        );
        let mut harness =
            TestHarness::create_with_size(default_property_set(), root, Size::new(120.0, 40.0));

        harness.focus_on(Some(radio_id));
        assert_render_snapshot!(harness, "radio_focus_focused");
    }
    #[test]
    fn edit_radio() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(StyleProperty::FontSize(20.0));
            let label = NewWidget::new_with_props(
                label,
                Properties::new().with(ContentColor::new(ACCENT_COLOR)),
            );
            let radio = NewWidget::new(RadioButton::from_label(true, label));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), radio, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let radio = NewWidget::new(RadioButton::new(false, "Hello world"));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), radio, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut radio| {
                RadioButton::set_checked(&mut radio, true);
                RadioButton::set_text(
                    &mut radio,
                    ArcStr::from("The quick brown fox jumps over the lazy dog"),
                );

                let mut label = RadioButton::label_mut(&mut radio);
                label.insert_prop(ContentColor::new(ACCENT_COLOR));
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
