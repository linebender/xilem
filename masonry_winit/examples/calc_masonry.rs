// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple calculator.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(
    variant_size_differences,
    clippy::single_match,
    reason = "Don't matter for example code"
)]

use std::str::FromStr;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, Action, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, StyleProperty, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetPod,
};
use masonry::dpi::LogicalSize;
use masonry::kurbo::{Point, Size};
use masonry::peniko::Color;
use masonry::peniko::color::AlphaColor;
use masonry::properties::{Background, Padding};
use masonry::smallvec::{SmallVec, smallvec};
use masonry::theme::default_property_set;
use masonry::vello::Scene;
use masonry::widgets::{Align, CrossAxisAlignment, Flex, Label, RootWidget, SizedBox};
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use tracing::{Span, trace, trace_span};
use winit::window::Window;

#[derive(Clone)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
    window_id: WindowId,
}

#[derive(Clone, Copy)]
enum CalcAction {
    Digit(u8),
    Op(char),
}

struct CalcButton {
    inner: WidgetPod<SizedBox>,
    action: CalcAction,
    base_color: Color,
    active_color: Color,
}

// ---

impl CalcState {
    fn digit(&mut self, digit: u8) {
        if !self.in_num {
            self.value.clear();
            self.in_num = true;
        }
        let ch = (b'0' + digit) as char;
        self.value.push(ch);
    }

    fn display(&mut self) {
        self.value = self.operand.to_string();
    }

    fn compute(&mut self) {
        if self.in_num {
            let operand2 = self.value.parse().unwrap_or(0.0);
            let result = match self.operator {
                '+' => Some(self.operand + operand2),
                '−' => Some(self.operand - operand2),
                '×' => Some(self.operand * operand2),
                '÷' => Some(self.operand / operand2),
                _ => None,
            };
            if let Some(result) = result {
                self.operand = result;
                self.display();
                self.in_num = false;
            }
        }
    }

    fn op(&mut self, op: char) {
        match op {
            '+' | '−' | '×' | '÷' | '=' => {
                self.compute();
                self.operand = self.value.parse().unwrap_or(0.0);
                self.operator = op;
                self.in_num = false;
            }
            '±' => {
                if self.in_num {
                    if self.value.starts_with('−') {
                        self.value = self.value[3..].to_string();
                    } else {
                        self.value = ["−", &self.value].concat();
                    }
                } else {
                    self.operand = -self.operand;
                    self.display();
                }
            }
            '.' => {
                if !self.in_num {
                    self.value = "0".to_string();
                    self.in_num = true;
                }
                if self.value.find('.').is_none() {
                    self.value.push('.');
                }
            }
            'c' => {
                self.value = "0".to_string();
                self.in_num = false;
            }
            'C' => {
                self.value = "0".to_string();
                self.operator = 'C';
                self.in_num = false;
            }
            '⌫' => {
                if self.in_num {
                    self.value.pop();
                    if self.value.is_empty() || self.value == "−" {
                        self.value = "0".to_string();
                        self.in_num = false;
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

impl CalcButton {
    fn new(inner: SizedBox, action: CalcAction, base_color: Color, active_color: Color) -> Self {
        Self {
            inner: WidgetPod::new(inner),
            action,
            base_color,
            active_color,
        }
    }
}

impl Widget for CalcButton {
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                if !ctx.is_disabled() {
                    let color = self.active_color;
                    // See `update` for why we use `mutate_later` here.
                    ctx.mutate_later(&mut self.inner, move |mut inner| {
                        SizedBox::set_background(&mut inner, color);
                    });
                    ctx.capture_pointer();
                    trace!("CalcButton {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::Up { .. } => {
                if ctx.is_pointer_capture_target() && !ctx.is_disabled() {
                    let color = self.base_color;
                    // See `update` for why we use `mutate_later` here.
                    ctx.mutate_later(&mut self.inner, move |mut inner| {
                        SizedBox::set_background(&mut inner, color);
                    });
                    ctx.submit_action(Action::Other(Box::new(self.action)));
                    trace!("CalcButton {:?} released", ctx.widget_id());
                }
            }
            _ => (),
        }
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
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.target() == ctx.widget_id() {
            match event.action {
                accesskit::Action::Click => {
                    ctx.submit_action(Action::Other(Box::new(self.action)));
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        // Masonry doesn't let us change a widget's attributes directly.
        // We use `mutate_later` to get a mutable reference to the inner widget
        // and change its border color. This is a simple way to implement a
        // "hovered" visual effect, but it's somewhat non-idiomatic compared to
        // implementing the effect inside the "paint" method.
        match event {
            Update::HoveredChanged(true) => {
                ctx.mutate_later(&mut self.inner, move |mut inner| {
                    SizedBox::set_border(&mut inner, Color::WHITE, 3.0);
                });
            }
            Update::HoveredChanged(false) => {
                ctx.mutate_later(&mut self.inner, move |mut inner| {
                    SizedBox::set_border(&mut inner, Color::TRANSPARENT, 3.0);
                });
            }
            _ => (),
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.inner);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.inner, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);

        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let _name = match self.action {
            CalcAction::Digit(digit) => digit.to_string(),
            CalcAction::Op(op) => op.to_string(),
        };
        // We may want to add a name if it doesn't interfere with the child label
        // ctx.current_node().set_name(name);
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.inner.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("CalcButton", id = ctx.widget_id().trace())
    }
}

impl AppDriver for CalcState {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        match action {
            Action::Other(payload) => match payload.downcast_ref::<CalcAction>().unwrap() {
                CalcAction::Digit(digit) => self.digit(*digit),
                CalcAction::Op(op) => self.op(*op),
            },
            _ => unreachable!(),
        }

        ctx.render_root(window_id).edit_root_widget(|mut root| {
            let mut root = root.downcast::<RootWidget>();
            let mut flex = RootWidget::child_mut(&mut root);
            let mut flex = flex.downcast();
            let mut label = Flex::child_mut(&mut flex, 1).unwrap();
            let mut label = label.downcast::<Label>();
            Label::set_text(&mut label, &*self.value);
        });
    }
}

// ---

fn op_button_with_label(op: char, label: String) -> CalcButton {
    const BLUE: Color = Color::from_rgb8(0x00, 0x8d, 0xdd);
    const LIGHT_BLUE: Color = Color::from_rgb8(0x5c, 0xc4, 0xff);

    CalcButton::new(
        SizedBox::new(Align::centered(
            Label::new(label).with_style(StyleProperty::FontSize(24.)),
        ))
        .background(BLUE)
        .expand(),
        CalcAction::Op(op),
        BLUE,
        LIGHT_BLUE,
    )
}

fn op_button(op: char) -> CalcButton {
    op_button_with_label(op, op.to_string())
}

fn digit_button(digit: u8) -> CalcButton {
    const GRAY: Color = Color::from_rgb8(0x3a, 0x3a, 0x3a);
    const LIGHT_GRAY: Color = Color::from_rgb8(0x71, 0x71, 0x71);
    CalcButton::new(
        SizedBox::new(Align::centered(
            Label::new(format!("{digit}")).with_style(StyleProperty::FontSize(24.)),
        ))
        .background(GRAY)
        .expand(),
        CalcAction::Digit(digit),
        GRAY,
        LIGHT_GRAY,
    )
}

fn flex_row(
    w1: impl Widget + 'static,
    w2: impl Widget + 'static,
    w3: impl Widget + 'static,
    w4: impl Widget + 'static,
) -> impl Widget {
    Flex::row()
        .gap(0.0)
        .with_flex_child(w1, 1.0)
        .with_spacer(1.0)
        .with_flex_child(w2, 1.0)
        .with_spacer(1.0)
        .with_flex_child(w3, 1.0)
        .with_spacer(1.0)
        .with_flex_child(w4, 1.0)
}

fn build_calc() -> impl Widget {
    let display = Label::new(String::new()).with_style(StyleProperty::FontSize(32.));
    Flex::column()
        .gap(0.0)
        .with_flex_spacer(0.2)
        .with_child(display)
        .with_flex_spacer(0.2)
        .cross_axis_alignment(CrossAxisAlignment::End)
        .with_flex_child(
            flex_row(
                op_button_with_label('c', "CE".to_string()),
                op_button('C'),
                op_button('⌫'),
                op_button('÷'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row(
                digit_button(7),
                digit_button(8),
                digit_button(9),
                op_button('×'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row(
                digit_button(4),
                digit_button(5),
                digit_button(6),
                op_button('−'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row(
                digit_button(1),
                digit_button(2),
                digit_button(3),
                op_button('+'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row(
                op_button('±'),
                digit_button(0),
                op_button('.'),
                op_button('='),
            ),
            1.0,
        )
}

fn main() {
    let window_size = LogicalSize::new(223., 300.);

    let window_attributes = Window::default_attributes()
        .with_title("Simple Calculator")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
        window_id: WindowId::next(),
    };

    let mut default_properties = default_property_set();
    default_properties
        .insert::<RootWidget, _>(Background::Color(AlphaColor::from_str("#794869").unwrap()));
    default_properties.insert::<RootWidget, _>(Padding::all(2.0));

    let event_loop = masonry_winit::app::EventLoop::with_user_event()
        .build()
        .unwrap();
    masonry_winit::app::run_with(
        event_loop,
        vec![(
            calc_state.window_id,
            window_attributes,
            Box::new(RootWidget::new(build_calc())),
        )],
        calc_state,
        default_properties,
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;
    use masonry::theme::default_property_set;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(default_property_set(), build_calc());
        assert_render_snapshot!(harness, "example_calc_masonry_initial");

        // TODO - Test clicking buttons
    }
}
