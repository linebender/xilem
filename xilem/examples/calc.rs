// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use xilem::{
    view::{button, flex, label, sized_box, Axis, FlexExt as _, FlexSpacer},
    EventLoop, WidgetView, Xilem,
};

#[derive(Copy, Clone)]
enum MathOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl MathOperator {
    fn as_str(&self) -> &'static str {
        match self {
            MathOperator::Add => "+",
            MathOperator::Subtract => "-",
            MathOperator::Multiply => "×",
            MathOperator::Divide => "÷",
        }
    }

    fn perform_op(&self, num1: f64, num2: f64) -> f64 {
        match self {
            MathOperator::Add => num1 + num2,
            MathOperator::Subtract => num1 - num2,
            MathOperator::Multiply => num1 * num2,
            MathOperator::Divide => num1 / num2,
        }
    }
}

struct AppData {
    current_num_index: usize,
    clear_current_entry_on_input: bool, // For instances of negation used on a result.
    numbers: [String; 2],
    result: Option<String>,
    operation: Option<MathOperator>,
}

impl AppData {
    fn get_current_number(&self) -> String {
        return self.numbers[self.current_num_index].clone()
    }

    fn set_current_number(&mut self, new_num: String) {
        self.numbers[self.current_num_index] = new_num;
    }

    fn clear_all(&mut self) {
        self.current_num_index = 0;
        self.result = None;
        self.operation = None;
        for num in self.numbers.iter_mut() { *num = "".into(); }
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
            if num.contains(".") {
                play_bell_sound();
                return;
            }
            // Make it so you don't end up with just a decimal point
            if num.is_empty() {
                num = "0".into();
            }
            num += "."
        } else {
            if num == "0" || num == "" {
                num = digit.to_string();
            } else {
                num += digit;
            }
        }
        self.set_current_number(num);
    }

    fn on_entered_operator(&mut self, operator: MathOperator) {
        self.clear_current_entry_on_input = false;
        if self.operation.is_some() && !self.numbers[1].is_empty() {
            if !self.result.is_some() {
                // All info is there to create a result, so calculate it.
                self.on_equals()
            }
            // There is a result present, so put that on the left.
            self.move_result_to_left();
            self.current_num_index = 1
        } else if self.current_num_index == 0 {
            if self.numbers[0].is_empty() {
                // Not ready yet. Left number needed.
                play_bell_sound();
                return;
            } else {
                self.current_num_index = 1;
            }
        }
        self.operation = Some(operator);

    }

    // For instances when you continue working with the prior result.
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
            play_bell_sound();
            return // Just abort.
        } else if self.result.is_some() {
            // Repeat the operation using the prior result on the left.
            self.numbers[0] = self.result.clone().unwrap()
        }
        self.current_num_index = 0;
        let num1 = self.numbers[0].parse::<f64>();
        let num2 = self.numbers[1].parse::<f64>();
        // Display format error or display the result of the operation.
        if num1.is_err() {
            self.result = Some(num1.err().unwrap().to_string())
        } else if num2.is_err() {
            self.result = Some(num2.err().unwrap().to_string())
        } else {
            self.result = Some(self.operation.unwrap().perform_op(num1.unwrap(), num2.unwrap()).to_string())
        }
    }

    fn on_delete(&mut self) {
        if self.result.is_some() {
            // Delete does not do anything with the result.
            play_bell_sound();
            return;
        }
        let mut num = self.get_current_number();
        if num.len() > 0 {
            num.remove(num.len() - 1);
            self.set_current_number(num);
        } else {
            play_bell_sound();
        }
    }

    fn negate(&mut self) {
        // If there is a result, negate that after clearing and moving it to the first number
        if self.result.is_some() {
            self.move_result_to_left()
        }
        let mut num = self.get_current_number();
        if num.is_empty() {
            play_bell_sound();
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

const DISPLAY_FONT_SIZE: f32 = 30.;
const GRID_GAP: f64 = 2.;
fn app_logic(data: &mut AppData) -> impl WidgetView<AppData> {
    flex((
        // Display
        flex((
            FlexSpacer::Flex(0.1),
            label(data.numbers[0].clone()).text_size(DISPLAY_FONT_SIZE),
            data.operation.clone().map(|operation| label(operation.as_str()).text_size(DISPLAY_FONT_SIZE)),
            label(data.numbers[1].clone()).text_size(DISPLAY_FONT_SIZE),
            data.result.clone().map(|_| label("=").text_size(DISPLAY_FONT_SIZE)),
            data.result.clone().map(|result| label(result).text_size(DISPLAY_FONT_SIZE)),
            FlexSpacer::Flex(0.1),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_alignment(MainAxisAlignment::Start)
            .gap(5.)
            .flex(1.0),
        FlexSpacer::Fixed(10.0),
        // Top row
        flex((
            sized_box(button("CE", |data: &mut AppData| {data.clear_entry()})).expand().flex(1.0),
            sized_box(button("C", |data: &mut AppData| {data.clear_all()})).expand().flex(1.0),
            sized_box(button("DEL", |data: &mut AppData| {data.on_delete()})).expand().flex(1.0),
            operator_button(MathOperator::Divide).flex(1.0),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
            .gap(GRID_GAP)
            .flex(1.0),
        // 7 8 9 X
        flex((
            digit_button("7").flex(1.0),
            digit_button("8").flex(1.0),
            digit_button("9").flex(1.0),
            operator_button(MathOperator::Multiply).flex(1.0),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
            .gap(GRID_GAP)
            .flex(1.0),
        // 4 5 6 -
        flex((
            digit_button("4").flex(1.0),
            digit_button("5").flex(1.0),
            digit_button("6").flex(1.0),
            operator_button(MathOperator::Subtract).flex(1.0),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
            .gap(GRID_GAP)
            .flex(1.0),
        // 1 2 3 +
        flex((
            digit_button("1").flex(1.0),
            digit_button("2").flex(1.0),
            digit_button("3").flex(1.0),
            operator_button(MathOperator::Add).flex(1.0),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
            .gap(GRID_GAP)
            .flex(1.0),
        // bottom row
        flex((
            sized_box(button("±", |data: &mut AppData| {data.negate()})).expand().flex(1.0),
            digit_button("0").flex(1.0),
            digit_button(".").flex(1.0),
            sized_box(button("=", |data: &mut AppData| {data.on_equals()})).expand().flex(1.0),
        ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
            .gap(GRID_GAP)
            .flex(1.0),
    ))
        .gap(GRID_GAP)
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .main_axis_alignment(MainAxisAlignment::End)
        .must_fill_major_axis(true)
}
fn operator_button(math_operator: MathOperator) -> impl WidgetView<AppData> {
    sized_box(
        button(
            math_operator.as_str(),
            move |data: &mut AppData| {
                data.on_entered_operator(math_operator)
            }
        )
    )
        .expand()
}

fn digit_button(digit: &'static str) -> impl WidgetView<AppData> {
    sized_box(
        button(digit, |data: &mut AppData| {
            data.on_entered_digit(digit)
        })
    )
        .expand()
}

fn play_bell_sound() {
    println!("\x07");
}

fn main() -> Result<(), EventLoopError> {
    let data = AppData {
        current_num_index: 0,
        clear_current_entry_on_input: false,
        numbers: ["".into(), "".into()],
        result: None,
        operation: None,
    };

    let app = Xilem::new(data, app_logic);
    let min_window_size = LogicalSize::new(200., 200.);
    let window_size = LogicalSize::new(400., 500.);
    let window_attributes = Window::default_attributes()
        .with_title("Calculator")
        .with_resizable(true)
        .with_min_inner_size(min_window_size)
        .with_inner_size(window_size);
    app.run_windowed_in(EventLoop::with_user_event(), window_attributes)?;
    Ok(())
}
