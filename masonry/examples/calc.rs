// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple calculator.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]
#![allow(variant_size_differences, clippy::single_match)]

use accesskit::{DefaultActionVerb, Role};
use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::dpi::LogicalSize;
use masonry::widget::{Align, CrossAxisAlignment, Flex, Label, RootWidget, SizedBox};
use masonry::{
    AccessCtx, AccessEvent, Action, BoxConstraints, Color, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
    WidgetPod,
};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::Scene;
use winit::window::Window;

#[derive(Clone)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
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
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.get_mut(&mut self.inner)
                        .set_background(self.active_color);
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("CalcButton {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::PointerUp(_, _) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    ctx.submit_action(Action::Other(Box::new(self.action)));
                    ctx.request_paint();
                    trace!("CalcButton {:?} released", ctx.widget_id());
                }
                ctx.get_mut(&mut self.inner).set_background(self.base_color);
                ctx.set_active(false);
            }
            _ => (),
        }
        self.inner.on_pointer_event(ctx, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.inner.on_text_event(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if event.target == ctx.widget_id() {
            match event.action {
                accesskit::Action::Default => {
                    ctx.submit_action(Action::Other(Box::new(self.action)));
                    ctx.request_paint();
                }
                _ => {}
            }
        }
        ctx.skip_child(&mut self.inner);
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::HotChanged(true) => {
                ctx.get_mut(&mut self.inner).set_border(Color::WHITE, 3.0);
                ctx.request_paint();
            }
            StatusChange::HotChanged(false) => {
                ctx.get_mut(&mut self.inner)
                    .set_border(Color::TRANSPARENT, 3.0);
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.inner.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.inner.layout(ctx, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.inner.paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        let _name = match self.action {
            CalcAction::Digit(digit) => digit.to_string(),
            CalcAction::Op(op) => op.to_string(),
        };
        // We may want to add a name if it doesn't interfere with the child label
        // ctx.current_node().set_name(name);
        ctx.current_node()
            .set_default_action_verb(DefaultActionVerb::Click);

        self.inner.accessibility(ctx);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.inner.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("CalcButton")
    }
}

impl AppDriver for CalcState {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::Other(payload) => match payload.downcast_ref::<CalcAction>().unwrap() {
                CalcAction::Digit(digit) => self.digit(*digit),
                CalcAction::Op(op) => self.op(*op),
            },
            _ => unreachable!(),
        }

        ctx.get_root::<RootWidget<Flex>>()
            .get_element()
            .child_mut(1)
            .unwrap()
            .downcast::<Label>()
            .set_text(&*self.value);
    }
}

// ---

fn op_button_with_label(op: char, label: String) -> CalcButton {
    const BLUE: Color = Color::rgb8(0x00, 0x8d, 0xdd);
    const LIGHT_BLUE: Color = Color::rgb8(0x5c, 0xc4, 0xff);

    CalcButton::new(
        SizedBox::new(Align::centered(Label::new(label).with_text_size(24.)))
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
    const GRAY: Color = Color::rgb8(0x3a, 0x3a, 0x3a);
    const LIGHT_GRAY: Color = Color::rgb8(0x71, 0x71, 0x71);
    CalcButton::new(
        SizedBox::new(Align::centered(
            Label::new(format!("{digit}")).with_text_size(24.),
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
    let display = Label::new(String::new()).with_text_size(32.0);
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

pub fn main() {
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
    };

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(build_calc()),
        calc_state,
    )
    .unwrap();
}
