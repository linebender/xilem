// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A button widget.

use std::any::TypeId;

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, Action, ArcStr, BoxConstraints, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::Size;
use crate::properties::types::Gradient;
use crate::properties::*;
use crate::theme;
use crate::util::{fill, stroke};
use crate::widgets::Label;

// --- MARK: CONSTANTS ---
const DEFAULT_BORDER_COLOR: BorderColor = BorderColor {
    color: theme::BORDER_DARK,
};
const DEFAULT_BORDER_WIDTH: BorderWidth = BorderWidth {
    width: theme::BUTTON_BORDER_WIDTH,
};
const DEFAULT_BORDER_RADII: CornerRadius = CornerRadius {
    radius: theme::BUTTON_BORDER_RADIUS,
};

// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const DEFAULT_PADDING: Padding = Padding { x: 8., y: 2. };

/// A button with a text label.
///
/// Emits [`Action::ButtonPressed`] when pressed.
///
#[doc = crate::include_screenshot!("button_hello.png", "Button with text label.")]
pub struct Button {
    label: WidgetPod<Label>,
}

// --- MARK: BUILDERS ---
impl Button {
    /// Create a new button with a text label.
    ///
    /// # Examples
    ///
    /// ```
    /// use masonry_core::widgets::Button;
    ///
    /// let button = Button::new("Increment");
    /// ```
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self::from_label(Label::new(text))
    }

    /// Create a new button with the provided [`Label`].
    ///
    /// # Examples
    ///
    /// ```
    /// use masonry_core::peniko::Color;
    /// use masonry_core::widgets::{Button, Label};
    ///
    /// let label = Label::new("Increment").with_brush(Color::new([0.5, 0.5, 0.5, 1.0]));
    /// let button = Button::from_label(label);
    /// ```
    pub fn from_label(label: Label) -> Self {
        Self {
            label: WidgetPod::new(label),
        }
    }

    /// Create a new button with the provided [`Label`] with a predetermined id.
    ///
    /// This constructor is useful for toolkits which use Masonry (such as Xilem).
    pub fn from_label_pod(label: WidgetPod<Label>) -> Self {
        Self { label }
    }
}

// --- MARK: WIDGETMUT ---
impl Button {
    /// Set the text.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    /// Get a mutable reference to the label.
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Button {
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    // Changes in pointer capture impact appearance, but not accessibility node
                    ctx.request_paint_only();
                    trace!("Button {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::PointerUp(button, _) => {
                if ctx.is_pointer_capture_target() && ctx.is_hovered() && !ctx.is_disabled() {
                    ctx.submit_action(Action::ButtonPressed(*button));
                    trace!("Button {:?} released", ctx.widget_id());
                }
                // Changes in pointer capture impact appearance, but not accessibility node
                ctx.request_paint_only();
            }
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.target() == ctx.widget_id() {
            match event.action {
                accesskit::Action::Click => {
                    ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) | Update::FocusChanged(_) | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut crate::core::RegisterCtx) {
        ctx.register_child(&mut self.label);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx, property_type: TypeId) {
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>().unwrap_or(&DEFAULT_BORDER_WIDTH);
        let padding = props.get::<Padding>().unwrap_or(&DEFAULT_PADDING);

        let initial_bc = bc;

        let bc = bc.loosen();
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        let label_size = ctx.run_layout(&mut self.label, &bc);
        let baseline = ctx.child_baseline_offset(&self.label);

        let size = label_size;
        let (size, baseline) = padding.layout_up(size, baseline);
        let (size, baseline) = border.layout_up(size, baseline);

        // TODO - Add MinimumSize property.
        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let mut size = size;
        size.height = size.height.max(theme::BORDERED_WIDGET_HEIGHT);

        // TODO - Figure out how to handle cases where label size doesn't fit bc.
        let size = initial_bc.constrain(size);
        let label_offset = (size.to_vec2() - label_size.to_vec2()) / 2.0;
        ctx.place_child(&mut self.label, label_offset.to_point());

        // TODO - pos = (size - label_size) / 2

        ctx.set_baseline_offset(baseline);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let is_pressed = ctx.is_pointer_capture_target() && !ctx.is_disabled();
        let is_hovered = ctx.is_hovered();
        let size = ctx.size();

        let border_color = props.get::<BorderColor>().unwrap_or(&DEFAULT_BORDER_COLOR);
        let border_width = props.get::<BorderWidth>().unwrap_or(&DEFAULT_BORDER_WIDTH);
        let border_radius = props.get::<CornerRadius>().unwrap_or(&DEFAULT_BORDER_RADII);

        // TODO - Add DEFAULT_BACKGROUND_GRADIENT constant.
        // Right now we can't because `.with_stops` isn't const-compatible.
        let bg_gradient =
            Gradient::new_linear(0.0).with_stops([theme::BUTTON_LIGHT, theme::BUTTON_DARK]);
        let bg_gradient = Background::Gradient(bg_gradient);
        let bg_gradient = props.get::<Background>().unwrap_or(&bg_gradient);

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        // TODO - Handle disabled and pressed bg with properties.
        let bg_gradient = if ctx.is_disabled() {
            &Background::Gradient(
                Gradient::new_linear(0.0)
                    .with_stops([theme::DISABLED_BUTTON_LIGHT, theme::DISABLED_BUTTON_DARK]),
            )
        } else if is_pressed {
            &Background::Gradient(
                Gradient::new_linear(0.0).with_stops([theme::BUTTON_DARK, theme::BUTTON_LIGHT]),
            )
        } else {
            bg_gradient
        };

        // TODO - Handle hovered color with properties.
        let border_color = if is_hovered && !ctx.is_disabled() {
            BorderColor {
                color: theme::BORDER_LIGHT,
            }
        } else {
            *border_color
        };

        let brush = bg_gradient.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, _props: &PropertiesRef<'_>, node: &mut Node) {
        // IMPORTANT: We don't want to merge this code in practice, because
        // the child label already has a 'name' property.
        // This is more of a proof of concept of `get_raw_ref()`.
        if false {
            let label = ctx.get_raw_ref(&self.label);
            let name = label.widget().text().as_ref().to_string();
            node.set_value(name);
        }
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.label.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Button", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::core::StyleProperty;
    use crate::testing::{TestHarness, TestWidgetExt, widget_ids};
    use crate::theme::PRIMARY_LIGHT;

    #[test]
    fn simple_button() {
        let [button_id] = widget_ids();
        let widget = Button::new("Hello").with_id(button_id);

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(widget, window_size);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "button_hello");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(button_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::ButtonPressed(PointerButton::Primary), button_id))
        );
    }

    #[test]
    fn edit_button() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_brush(PRIMARY_LIGHT)
                .with_style(StyleProperty::FontSize(20.0));
            let button = Button::from_label(label);

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let button = Button::new("Hello world");

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut button| {
                let mut button = button.downcast::<Button>();
                Button::set_text(&mut button, "The quick brown fox jumps over the lazy dog");

                let mut label = Button::label_mut(&mut button);
                Label::set_brush(&mut label, PRIMARY_LIGHT);
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }

    #[test]
    fn set_properties() {
        let red = crate::palette::css::RED;
        let button = Button::new("Some random text");

        let window_size = Size::new(200.0, 80.0);
        let mut harness = TestHarness::create_with_size(button, window_size);

        harness.edit_root_widget(|mut button| {
            let mut button = button.downcast::<Button>();

            button.insert_prop(BorderColor { color: red });
            button.insert_prop(BorderWidth { width: 5.0 });
            button.insert_prop(CornerRadius { radius: 20.0 });
            button.insert_prop(Padding { x: 8.0, y: 3.0 });

            let mut label = Button::label_mut(&mut button);
            Label::set_brush(&mut label, red);
        });

        assert_render_snapshot!(harness, "button_set_properties");
    }
}
