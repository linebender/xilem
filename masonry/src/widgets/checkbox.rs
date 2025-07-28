// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A checkbox widget.

use std::any::TypeId;

use accesskit::{Node, Role, Toggled};
use tracing::{Span, trace, trace_span};
use ui_events::keyboard::Key;
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Cap, Dashes, Join, Size, Stroke};
use vello::peniko::Color;

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

/// A checkbox that can be toggled.
///
#[doc = include_screenshot!("checkbox_hello_checked.png", "Checkbox with checked state.")]
///
/// Emits [`CheckboxToggled`] when toggled.
pub struct Checkbox {
    checked: bool,
    // FIXME - Remove label child, have this widget only be a box with a checkmark.
    label: WidgetPod<Label>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Self {
        Self {
            checked,
            label: WidgetPod::new(Label::new(text)),
        }
    }

    /// Create a new `Checkbox` with the given label.
    pub fn from_label(checked: bool, label: NewWidget<Label>) -> Self {
        Self {
            checked,
            label: label.to_pod(),
        }
    }
}

// --- MARK: WIDGETMUT
impl Checkbox {
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

/// The action type emitted by [`Checkbox`] when it is toggled.
///
/// The field is the toggle state (i.e. true is "checked").
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CheckboxToggled(pub bool);

// --- MARK: IMPL WIDGET
impl Widget for Checkbox {
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    // Checked state impacts appearance and accessibility node
                    ctx.request_render();
                    trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::Up { .. } => {
                if ctx.is_active() && ctx.is_hovered() && !ctx.is_disabled() {
                    self.checked = !self.checked;
                    ctx.submit_action(CheckboxToggled(self.checked));
                    trace!("Checkbox {:?} released", ctx.widget_id());
                }
                // Checked state impacts appearance and accessibility node
                ctx.request_render();
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
                    self.checked = !self.checked;
                    ctx.submit_action(CheckboxToggled(self.checked));
                    // Checked state impacts appearance and accessibility node
                    ctx.request_render();
                }
            }
            _ => (),
        }
    }

    fn accepts_focus(&self) -> bool {
        // Checkbox can be tab-focused...
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
                self.checked = !self.checked;
                ctx.submit_action(CheckboxToggled(self.checked));
                // Checked state impacts appearance and accessibility node
                ctx.request_render();
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
        let is_pressed = ctx.is_active() && !ctx.is_disabled();
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

        let mut border_color = if is_hovered && !ctx.is_disabled() {
            &props.get::<HoveredBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };
        // FIXME - Handle this properly
        if ctx.is_focus_target() {
            border_color = &BorderColor {
                color: Color::WHITE,
            };
        }
        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);

        if self.checked {
            let checkmark_width = props.get::<CheckmarkStrokeWidth>();
            let brush = if ctx.is_disabled() {
                &props.get::<DisabledCheckmarkColor>().0
            } else {
                props.get::<CheckmarkColor>()
            };

            // Paint the checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let style = Stroke {
                width: checkmark_width.width,
                join: Join::Round,
                miter_limit: 10.0,
                start_cap: Cap::Round,
                end_cap: Cap::Round,
                dash_pattern: Dashes::default(),
                dash_offset: 0.0,
            };
            scene.stroke(&style, Affine::IDENTITY, brush.color, None, &path);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::CheckBox
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
        trace_span!("Checkbox", id = id.trace())
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
    use ui_events::keyboard::NamedKey;

    use super::*;
    use crate::core::{Properties, StyleProperty};
    use crate::properties::TextColor;
    use crate::testing::{TestHarness, assert_render_snapshot, widget_ids};
    use crate::theme::{ACCENT_COLOR, default_property_set};

    #[test]
    fn simple_checkbox() {
        let [checkbox_id] = widget_ids();
        let widget = NewWidget::new_with_id(Checkbox::new(false, "Hello"), checkbox_id);

        let window_size = Size::new(100.0, 40.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "checkbox_hello_unchecked");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(checkbox_id);
        assert_eq!(
            harness.pop_action::<CheckboxToggled>(),
            Some((CheckboxToggled(true), checkbox_id))
        );

        assert_render_snapshot!(harness, "checkbox_hello_checked");

        harness.focus_on(None);
        harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(checkbox_id));

        harness.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
        harness.process_text_event(TextEvent::key_up(Key::Character(" ".into())));
        assert_eq!(
            harness.pop_action::<CheckboxToggled>(),
            Some((CheckboxToggled(false), checkbox_id))
        );
    }

    #[test]
    fn edit_checkbox() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(StyleProperty::FontSize(20.0));
            let label = NewWidget::new_with_props(
                label,
                Properties::new().with(TextColor::new(ACCENT_COLOR)),
            );
            let checkbox = NewWidget::new(Checkbox::from_label(true, label));

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                checkbox,
                Size::new(50.0, 50.0),
            );

            harness.render()
        };

        let image_2 = {
            let checkbox = NewWidget::new(Checkbox::new(false, "Hello world"));

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                checkbox,
                Size::new(50.0, 50.0),
            );

            harness.edit_root_widget(|mut checkbox| {
                Checkbox::set_checked(&mut checkbox, true);
                Checkbox::set_text(
                    &mut checkbox,
                    ArcStr::from("The quick brown fox jumps over the lazy dog"),
                );

                let mut label = Checkbox::label_mut(&mut checkbox);
                label.insert_prop(TextColor::new(ACCENT_COLOR));
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
