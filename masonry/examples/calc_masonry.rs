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
use std::sync::mpsc;

use masonry::core::{
    ErasedAction, NewWidget, Properties, Property, StyleProperty, Widget, WidgetId, WidgetOptions,
};
use masonry::dpi::LogicalSize;
use masonry::peniko::Color;
use masonry::peniko::color::AlphaColor;
use masonry::properties::types::{AsUnit, CrossAxisAlignment};
use masonry::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, HoveredBorderColor, Padding,
};
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Grid, GridParams, Label};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};

#[derive(Clone)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
    window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
/// The Action type for `CalcButton`
enum CalcAction {
    Digit(u8),
    Op(char),
    None,
}

// We use CalcAction as a property and store it alongside buttons
impl Property for CalcAction {
    fn static_default() -> &'static Self {
        &Self::None
    }
}

impl Default for CalcAction {
    fn default() -> Self {
        *Property::static_default()
    }
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

impl AppDriver for CalcState {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        let Some(source) = ctx.render_root(window_id).get_widget(widget_id) else {
            return;
        };
        let Some(button) = source.downcast::<Button>() else {
            return;
        };
        let Ok(_action) = action.downcast::<ButtonPress>() else {
            return;
        };

        match button.get_prop::<CalcAction>() {
            CalcAction::Digit(digit) => self.digit(*digit),
            CalcAction::Op(op) => self.op(*op),
            CalcAction::None => (),
        }

        ctx.render_root(window_id).edit_root_widget(|mut root| {
            let mut grid = root.downcast();
            let mut flex = Grid::child_mut(&mut grid, 0);
            let mut flex = flex.downcast();
            let mut label = Flex::child_mut(&mut flex, 1).unwrap();
            let mut label = label.downcast::<Label>();
            Label::set_text(&mut label, &*self.value);
        });
    }
}

// ---

fn op_button_with_label(op: char, label: String) -> NewWidget<Button> {
    const BLUE: Color = Color::from_rgb8(0x00, 0x8d, 0xdd);
    const LIGHT_BLUE: Color = Color::from_rgb8(0x5c, 0xc4, 0xff);

    let button = Button::new(
        Label::new(label)
            .with_style(StyleProperty::FontSize(24.))
            .with_auto_id(),
    );

    NewWidget::new_with(
        button,
        WidgetId::next(),
        WidgetOptions::default(),
        Properties::new()
            .with(Background::Color(BLUE))
            .with(ActiveBackground(Background::Color(LIGHT_BLUE)))
            .with(HoveredBorderColor(BorderColor::new(Color::WHITE)))
            .with(BorderColor::new(Color::TRANSPARENT))
            .with(BorderWidth::all(2.0))
            .with(CalcAction::Op(op)),
    )
}

fn op_button(op: char) -> NewWidget<Button> {
    op_button_with_label(op, op.to_string())
}

fn digit_button(digit: u8) -> NewWidget<Button> {
    const GRAY: Color = Color::from_rgb8(0x3a, 0x3a, 0x3a);
    const LIGHT_GRAY: Color = Color::from_rgb8(0x71, 0x71, 0x71);

    let button = Button::new(
        Label::new(format!("{digit}"))
            .with_style(StyleProperty::FontSize(24.))
            .with_auto_id(),
    );

    NewWidget::new_with(
        button,
        WidgetId::next(),
        WidgetOptions::default(),
        Properties::new()
            .with(Background::Color(GRAY))
            .with(ActiveBackground(Background::Color(LIGHT_GRAY)))
            .with(HoveredBorderColor(BorderColor::new(Color::WHITE)))
            .with(BorderColor::new(Color::TRANSPARENT))
            .with(BorderWidth::all(2.0))
            .with(CalcAction::Digit(digit)),
    )
}

/// Build the widget tree
pub fn build_calc() -> NewWidget<impl Widget> {
    let display = Label::new(String::new()).with_style(StyleProperty::FontSize(32.));
    let display = Flex::column()
        .with_flex_spacer(1.)
        .with_child(display.with_auto_id())
        .with_flex_spacer(1.)
        .cross_axis_alignment(CrossAxisAlignment::End);

    fn button_params(x: i32, y: i32) -> GridParams {
        GridParams::new(x, y, 1, 1)
    }

    let root_widget = Grid::with_dimensions(4, 6)
        .with_spacing(1.px())
        .with_child(display.with_auto_id(), GridParams::new(0, 0, 4, 1))
        .with_child(
            op_button_with_label('c', "CE".to_string()),
            button_params(0, 1),
        )
        .with_child(op_button('C'), button_params(1, 1))
        .with_child(op_button('⌫'), button_params(2, 1))
        .with_child(op_button('÷'), button_params(3, 1))
        .with_child(digit_button(7), button_params(0, 2))
        .with_child(digit_button(8), button_params(1, 2))
        .with_child(digit_button(9), button_params(2, 2))
        .with_child(op_button('×'), button_params(3, 2))
        .with_child(digit_button(4), button_params(0, 3))
        .with_child(digit_button(5), button_params(1, 3))
        .with_child(digit_button(6), button_params(2, 3))
        .with_child(op_button('−'), button_params(3, 3))
        .with_child(digit_button(1), button_params(0, 4))
        .with_child(digit_button(2), button_params(1, 4))
        .with_child(digit_button(3), button_params(2, 4))
        .with_child(op_button('+'), button_params(3, 4))
        .with_child(op_button('±'), button_params(0, 5))
        .with_child(digit_button(0), button_params(1, 5))
        .with_child(op_button('.'), button_params(2, 5))
        .with_child(op_button('='), button_params(3, 5));

    NewWidget::new_with_props(
        root_widget,
        Properties::new()
            .with(Background::Color(AlphaColor::from_str("#794869").unwrap()))
            .with(Padding::all(2.0)),
    )
}

fn main() {
    let window_size = LogicalSize::new(223., 300.);

    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("Simple Calculator")
        .with_resizable(true)
        .with_min_surface_size(window_size);

    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
        window_id: WindowId::next(),
    };

    let event_loop = masonry_winit::app::EventLoop::new().unwrap();
    let (event_sender, event_receiver) = mpsc::channel::<masonry_winit::app::MasonryUserEvent>();

    masonry_winit::app::run_with(
        event_loop,
        event_sender,
        event_receiver,
        vec![NewWindow::new_with_id(
            calc_state.window_id,
            window_attributes,
            build_calc().erased(),
        )],
        calc_state,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::{TestHarness, TestHarnessParams, assert_render_snapshot};

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            build_calc(),
            TestHarnessParams::default(),
        );
        assert_render_snapshot!(harness, "example_calc_masonry_initial");

        // TODO - Test clicking buttons
    }
}
