// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A simple calculator example
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]

use masonry::properties::types::{AsUnit, CrossAxisAlignment, Length, MainAxisAlignment};
use masonry::widgets::GridParams;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    Axis, Flex, FlexSequence, FlexSpacer, GridExt, GridSequence, Label, button, flex, grid, label,
    sized_box,
};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem, palette};

#[derive(Copy, Clone)]
enum MathOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl MathOperator {
    fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "\u{2212}",
            Self::Multiply => "×",
            Self::Divide => "÷",
        }
    }

    fn perform_op(self, num1: f64, num2: f64) -> f64 {
        match self {
            Self::Add => num1 + num2,
            Self::Subtract => num1 - num2,
            Self::Multiply => num1 * num2,
            Self::Divide => num1 / num2,
        }
    }
}

struct Calculator {
    current_num_index: usize,
    clear_current_entry_on_input: bool, // For instances of negation used on a result.
    numbers: [String; 2],
    result: Option<String>,
    operation: Option<MathOperator>,
}

impl Calculator {
    fn get_current_number(&self) -> String {
        self.current_number().to_string()
    }

    fn current_number(&self) -> &str {
        &self.numbers[self.current_num_index]
    }

    fn set_current_number(&mut self, new_num: String) {
        self.numbers[self.current_num_index] = new_num;
    }

    fn clear_all(&mut self) {
        self.current_num_index = 0;
        self.result = None;
        self.operation = None;
        for num in self.numbers.iter_mut() {
            *num = "".into();
        }
    }

    fn clear_entry(&mut self) {
        self.clear_current_entry_on_input = false;
        if self.result.is_some() {
            self.clear_all();
            return;
        }
        self.set_current_number("".into());
    }

    fn on_entered_digit(&mut self, digit: &str) {
        if self.result.is_some() {
            self.clear_all();
        } else if self.clear_current_entry_on_input {
            self.clear_entry();
        }
        let mut num = self.get_current_number();
        // Special case: Don't allow more than one decimal.
        if digit == "." {
            if num.contains('.') {
                // invalid action
                return;
            }
            // Make it so you don't end up with just a decimal point
            if num.is_empty() {
                num = "0".into();
            }
            num += ".";
        } else if num == "0" || num.is_empty() {
            num = digit.to_string();
        } else {
            num += digit;
        }
        self.set_current_number(num);
    }

    fn on_entered_operator(&mut self, operator: MathOperator) {
        self.clear_current_entry_on_input = false;
        if self.operation.is_some() && !self.numbers[1].is_empty() {
            if self.result.is_none() {
                // All info is there to create a result, so calculate it.
                self.on_equals();
            }
            // There is a result present, so put that on the left.
            self.move_result_to_left();
            self.current_num_index = 1;
        } else if self.current_num_index == 0 {
            if self.numbers[0].is_empty() {
                // Not ready yet. Left number needed.
                // invalid action
                return;
            } else {
                self.current_num_index = 1;
            }
        }
        self.operation = Some(operator);
    }

    /// For instances when you continue working with the prior result.
    fn move_result_to_left(&mut self) {
        self.clear_current_entry_on_input = true;
        self.numbers[0] = self.result.clone().expect("expected result");
        self.numbers[1] = "".into();
        self.operation = None;
        self.current_num_index = 0; // Moved to left
        self.result = None; // It's moved, so remove the result.
    }

    fn on_equals(&mut self) {
        // Requires both numbers be present
        if self.numbers[0].is_empty() || self.numbers[1].is_empty() {
            // invalid action
            return; // Just abort.
        } else if self.result.is_some() {
            // Repeat the operation using the prior result on the left.
            self.numbers[0] = self.result.clone().unwrap();
        }
        self.current_num_index = 0;
        let num1 = self.numbers[0].parse::<f64>();
        let num2 = self.numbers[1].parse::<f64>();
        // Display format error or display the result of the operation.
        self.result = Some(match (num1, num2) {
            (Ok(num1), Ok(num2)) => self.operation.unwrap().perform_op(num1, num2).to_string(),
            (Err(err), _) => err.to_string(),
            (_, Err(err)) => err.to_string(),
        });
    }

    fn on_delete(&mut self) {
        if self.result.is_some() {
            // Delete does not do anything with the result. Invalid action.
            return;
        }
        let mut num = self.get_current_number();
        if !num.is_empty() {
            num.remove(num.len() - 1);
            self.set_current_number(num);
        } // else, invalid action
    }

    fn negate(&mut self) {
        // If there is a result, negate that after clearing and moving it to the first number
        if self.result.is_some() {
            self.move_result_to_left();
        }
        let mut num = self.get_current_number();
        if num.is_empty() {
            // invalid action
            return;
        }
        if num.starts_with('-') {
            num.remove(0);
        } else {
            num = format!("-{num}");
        }
        self.set_current_number(num);
    }
}

fn num_row(nums: [&'static str; 3], row: i32) -> impl GridSequence<Calculator> {
    let mut views: Vec<_> = vec![];
    for (i, num) in nums.iter().enumerate() {
        views.push(digit_button(num).grid_pos(i as i32, row));
    }
    views
}

const DISPLAY_FONT_SIZE: f32 = 30.;
const GRID_GAP: Length = Length::const_px(2.);
fn app_logic(data: &mut Calculator) -> impl WidgetView<Calculator> + use<> {
    grid(
        (
            // Display
            centered_flex_row((
                FlexSpacer::Flex(0.1),
                display_label(data.numbers[0].as_ref()),
                data.operation
                    .map(|operation| display_label(operation.as_str())),
                display_label(data.numbers[1].as_ref()),
                data.result.is_some().then(|| display_label("=")),
                data.result
                    .as_ref()
                    .map(|result| display_label(result.as_ref())),
                FlexSpacer::Flex(0.1),
            ))
            .grid_item(GridParams::new(0, 0, 4, 1)),
            // Top row
            expanded_button(
                label("CE").color(if data.get_current_number().is_empty() {
                    palette::css::MEDIUM_VIOLET_RED
                } else {
                    palette::css::WHITE
                }),
                Calculator::clear_entry,
            )
            .grid_pos(0, 1),
            expanded_button("C", Calculator::clear_all).grid_pos(1, 1),
            expanded_button("DEL", Calculator::on_delete).grid_pos(2, 1),
            operator_button(MathOperator::Divide).grid_pos(3, 1),
            num_row(["7", "8", "9"], 2),
            operator_button(MathOperator::Multiply).grid_pos(3, 2),
            num_row(["4", "5", "6"], 3),
            operator_button(MathOperator::Subtract).grid_pos(3, 3),
            num_row(["1", "2", "3"], 4),
            operator_button(MathOperator::Add).grid_pos(3, 4),
            // bottom row
            expanded_button("±", Calculator::negate).grid_pos(0, 5),
            digit_button("0").grid_pos(1, 5),
            expanded_button(".", |data: &mut Calculator| {
                data.on_entered_digit(".");
            })
            .grid_pos(2, 5),
            expanded_button("=", Calculator::on_equals).grid_pos(3, 5),
        ),
        4,
        6,
    )
    .spacing(GRID_GAP)
}

/// Creates a horizontal centered flex row designed for the display portion of the calculator.
pub fn centered_flex_row<State, Seq: FlexSequence<State>>(sequence: Seq) -> Flex<Seq, State> {
    flex(sequence)
        .direction(Axis::Horizontal)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Start)
        .gap(5.px())
}

/// Returns a label intended to be used in the calculator's top display.
/// The default text size is out of proportion for this use case.
fn display_label(text: &str) -> impl WidgetView<Calculator> + use<> {
    label(text).text_size(DISPLAY_FONT_SIZE)
}

/// Returns a button contained in an expanded box. Useful for the buttons so that
/// they take up all available space in flex containers.
fn expanded_button(
    text: impl Into<Label>,
    callback: impl Fn(&mut Calculator) + Send + Sync + 'static,
) -> impl WidgetView<Calculator> {
    const BLUE: Color = Color::from_rgb8(0x00, 0x8d, 0xdd);

    sized_box(
        button(text, callback)
            .background_color(BLUE)
            .corner_radius(10.)
            .border_color(Color::TRANSPARENT)
            .hovered_border_color(Color::WHITE),
    )
    .expand()
}

/// Returns an expanded button that triggers the calculator's operator handler,
/// `on_entered_operator()`.
fn operator_button(math_operator: MathOperator) -> impl WidgetView<Calculator> {
    expanded_button(math_operator.as_str(), move |data: &mut Calculator| {
        data.on_entered_operator(math_operator);
    })
}

/// A button which adds `digit` to the current input when pressed
fn digit_button(digit: &'static str) -> impl WidgetView<Calculator> {
    const GRAY: Color = Color::from_rgb8(0x3a, 0x3a, 0x3a);

    sized_box(
        button(digit, |data: &mut Calculator| {
            data.on_entered_digit(digit);
        })
        .background_color(GRAY)
        .corner_radius(10.)
        .border_color(Color::TRANSPARENT),
    )
    .expand()
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Calculator {
        current_num_index: 0,
        clear_current_entry_on_input: false,
        numbers: ["".into(), "".into()],
        result: None,
        operation: None,
    };

    let min_window_size = LogicalSize::new(200., 200.);
    let window_size = LogicalSize::new(400., 500.);
    let window_options = WindowOptions::new("Calculator").with_min_inner_size(min_window_size);
    // On iOS, winit has unsensible handling of `inner_size`
    // See https://github.com/rust-windowing/winit/issues/2308 for more details
    #[cfg(not(target_os = "ios"))]
    let window_options = window_options.with_initial_inner_size(window_size);
    let app = Xilem::new_simple(data, app_logic, window_options);
    app.run_in(event_loop)?;
    Ok(())
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::builder())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
