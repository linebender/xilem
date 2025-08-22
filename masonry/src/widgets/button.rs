// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A button widget.

use std::any::TypeId;
use std::sync::Arc;

use accesskit::{Node, Role};
use tracing::{Span, trace, trace_span};
use ui_events::pointer::{PointerButton, PointerButtonEvent};
use vello::Scene;
use vello::kurbo::{Affine, Size};
use vello::peniko::Color;

use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, HoveredBorderColor, Padding,
};
use crate::theme;
use crate::util::{fill, include_screenshot, stroke};

use super::Label;

/// A button with a child widget.
///
/// Emits [`ButtonPress`] when pressed.
///
#[doc = include_screenshot!("button_hello.png", "Button with text label.")]
pub struct Button {
    child: WidgetPod<dyn Widget>,
}

// --- MARK: BUILDERS
impl Button {
    /// Create a new button with a child widget.
    ///
    /// The child widget probably shouldn't be interactive,
    /// to avoid behaviour which might be confusing to the user.
    ///
    /// # Examples
    ///
    /// ```
    /// use masonry::widgets::{Button, Label};
    /// use masonry::core::Widget;
    ///
    /// let button = Button::new(Label::new("Increment").with_auto_id());
    /// ```
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
        }
    }

    /// Create a new button with a label widget.
    ///
    /// # Examples
    ///
    /// ```
    /// use masonry::widgets::Button;
    /// use masonry::core::Widget;
    ///
    /// let button = Button::with_text("Increment");
    /// ```
    pub fn with_text(text: impl Into<Arc<str>>) -> Self {
        Self::new(Label::new(text).with_auto_id())
    }
}

// --- MARK: WIDGETMUT
impl Button {
    /// Get a mutable reference to the label.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

/// A button was pressed.
#[derive(PartialEq, Debug)]
pub struct ButtonPress {
    /// The pointer button that has been pressed.
    ///
    /// Can be `None` when using for example the keyboard or a touch screen.
    pub button: Option<PointerButton>,
}

// --- MARK: IMPL WIDGET
impl Widget for Button {
    type Action = ButtonPress;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                ctx.capture_pointer();
                // Changes in pointer capture impact appearance, but not accessibility node
                ctx.request_paint_only();
                trace!("Button {:?} pressed", ctx.widget_id());
            }
            PointerEvent::Up(PointerButtonEvent { button, .. }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<Self::Action>(ButtonPress { button: *button });
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
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(event) if event.state.is_up() => {
                if matches!(&event.key, Key::Character(c) if c == " ")
                    || event.key == Key::Named(NamedKey::Enter)
                {
                    ctx.submit_action::<Self::Action>(ButtonPress { button: None });
                }
            }
            _ => (),
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            accesskit::Action::Click => {
                ctx.submit_action::<Self::Action>(ButtonPress { button: None });
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
        ctx.register_child(&mut self.child);
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

        let mut border_color = if is_hovered {
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
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let border_radius = props.get::<CornerRadius>();
        let shadow = props.get::<BoxShadow>();

        let shadow_rect = shadow.shadow_rect(size, border_radius);

        shadow.paint(scene, Affine::IDENTITY, shadow_rect);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
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

    fn accepts_focus(&self) -> bool {
        // Buttons can be tab-focused...
        true
    }

    fn accepts_text_input(&self) -> bool {
        // But they still aren't text areas.
        false
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Button", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::{TestHarnessParams, assert_failing_render_snapshot};

    use super::*;
    use crate::core::{PointerButton, Properties, StyleProperty};
    use crate::properties::ContentColor;
    use crate::properties::types::AsUnit;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, default_property_set};
    use crate::widgets::{Grid, GridParams, Label};

    #[test]
    fn simple_button() {
        let widget = NewWidget::new(Button::with_text("Hello"));

        let window_size = Size::new(100.0, 40.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);
        let button_id = harness.root_id();

        assert_render_snapshot!(harness, "button_hello");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(button_id);
        assert_eq!(
            harness.pop_action::<ButtonPress>(),
            Some((
                ButtonPress {
                    button: Some(PointerButton::Primary)
                },
                button_id
            ))
        );

        // Check that Tab focuses on the widget
        harness.focus_on(None);
        harness.press_tab_key(false);
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(button_id));

        harness.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
        harness.process_text_event(TextEvent::key_up(Key::Character(" ".into())));
        assert_eq!(
            harness.pop_action(),
            Some((ButtonPress { button: None }, button_id))
        );
    }

    #[test]
    fn edit_button() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(StyleProperty::FontSize(20.0));
            let label = NewWidget::new_with_props(
                label,
                Properties::new().with(ContentColor::new(ACCENT_COLOR)),
            );

            let button = NewWidget::new(Button::new(label));

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                button,
                Size::new(50.0, 50.0),
            );

            harness.render()
        };

        let image_2 = {
            let button = NewWidget::new(Button::with_text("Hello world"));

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                button,
                Size::new(50.0, 50.0),
            );

            harness.edit_root_widget(|mut button| {
                let mut label = Button::child_mut(&mut button);
                let mut label = label.downcast();

                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");

                label.insert_prop(ContentColor::new(ACCENT_COLOR));
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
        let button = NewWidget::new(Button::with_text("Some random text"));

        let window_size = Size::new(200.0, 80.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), button, window_size);

        harness.edit_root_widget(|mut button| {
            button.insert_prop(BorderColor { color: red });
            button.insert_prop(BorderWidth { width: 5.0 });
            button.insert_prop(CornerRadius { radius: 20.0 });
            button.insert_prop(Padding::from_vh(3., 8.));

            let mut label = Button::child_mut(&mut button);
            label.insert_prop(ContentColor::new(red));
        });

        assert_render_snapshot!(harness, "button_set_properties");
    }

    #[test]
    fn with_shadows() {
        use crate::palette::css::ORANGE;

        let grid = Grid::with_dimensions(2, 2)
            .with_spacing(40.px())
            .with_child(
                Button::with_text("A").with_auto_id(),
                GridParams::new(0, 0, 1, 1),
            )
            .with_child(
                Button::with_text("B").with_auto_id(),
                GridParams::new(1, 0, 1, 1),
            )
            .with_child(
                Button::with_text("C").with_auto_id(),
                GridParams::new(0, 1, 1, 1),
            )
            .with_child(
                Button::with_text("D").with_auto_id(),
                GridParams::new(1, 1, 1, 1),
            );
        let root_widget =
            NewWidget::new_with_props(grid, Properties::new().with(Padding::all(20.0)));

        let mut test_params = TestHarnessParams::default();
        test_params.window_size = Size::new(300.0, 300.0);
        test_params.screenshot_tolerance = 32;
        let mut harness =
            TestHarness::create_with(default_property_set(), root_widget, test_params);

        harness.edit_root_widget(|mut grid| {
            {
                let mut button = Grid::child_mut(&mut grid, 0);
                let mut button = button.downcast::<Button>();
                button.insert_prop(BoxShadow::new(ORANGE, (10., 10.)));
            }

            {
                let mut button = Grid::child_mut(&mut grid, 1);
                let mut button = button.downcast::<Button>();
                button.insert_prop(BoxShadow::new(ORANGE, (-10., 10.)).blur(5.0));
            }

            {
                let mut button = Grid::child_mut(&mut grid, 2);
                let mut button = button.downcast::<Button>();
                button.insert_prop(BoxShadow::new(ORANGE, (-10., -10.)).blur(-5.0));
            }

            {
                let mut button = Grid::child_mut(&mut grid, 3);
                let mut button = button.downcast::<Button>();
                button.insert_prop(BoxShadow::new(ORANGE, (0., 0.)).blur(5.0));
            }
        });

        assert_render_snapshot!(harness, "button_shadows");

        // Check that slightly changing the blur radius makes the screenshot test fail.
        // If it doesn't, the screenshot_tolerance param is too high.
        harness.edit_root_widget(|mut grid| {
            // Copy-pasted from second case above.
            {
                let mut button = Grid::child_mut(&mut grid, 1);
                let mut button = button.downcast::<Button>();
                button.insert_prop(BoxShadow::new(ORANGE, (-10., 10.)).blur(2.5));
            }
        });
        assert_failing_render_snapshot!(harness, "button_shadows");
    }
}
