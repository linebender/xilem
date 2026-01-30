// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing the capabilities of the Slider widget.
//!
//! This demo presents a simple "Color Picker" control panel, where each color
//! channel (Red, Green, Blue, Alpha) can be adjusted using a slider.
//! The result is displayed in a preview box, demonstrating how widgets can
//! react to and manipulate a shared application state.

use masonry::kurbo::Axis;
use masonry::layout::{AsUnit, Dim};
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, BarColor, ThumbColor, ThumbRadius};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    FlexExt, FlexSpacer, MainAxisAlignment, checkbox, flex, flex_col, flex_row, label, sized_box,
    slider, text_button,
};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

// --- Application State ---
/// Represents the state of our color picker application.
struct AppState {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    /// Stores the alpha value before it's locked to opaque.
    saved_alpha: f64,
    /// A flag to lock the alpha channel to fully opaque.
    use_transparency: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            red: 95.0,
            green: 10.0,
            blue: 60.0,
            alpha: 100.0,
            saved_alpha: 100.0,
            use_transparency: true,
        }
    }
}

impl AppState {
    /// Resets all values to their default state.
    fn reset(&mut self) {
        *self = Self::default();
    }
}

// --- Reusable View Components ---

/// A reusable view component that encapsulates a label, a slider, and a value display.
/// This helps keep the main `app_logic` clean and avoids code repetition.
fn control_slider<F>(
    label_text: &'static str,
    value: f64,
    u_value: u8,
    on_change: F,
) -> impl WidgetView<Edit<AppState>>
where
    F: Fn(&mut AppState, f64) + Send + Sync + 'static,
{
    flex_row((
        label(label_text).width(40.px()),
        slider::<Edit<AppState>, _, _>(0.0, 100.0, value, on_change).width(200.px()),
        label(format!("{:.0}% [{}]", value, u_value)).width(60.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(10.0.px())
}

/// Convert 0-100 to 0-255 u8 value.
fn perc_to_u8(value: f64) -> u8 {
    #![expect(clippy::cast_possible_truncation, reason = "This will never truncate")]
    (value * 2.56).clamp(0.0, 255.0).round() as u8
}

// --- Main UI Logic ---

/// The main logic for building the user interface.
fn app_logic(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    let color_red = perc_to_u8(state.red);
    let color_green = perc_to_u8(state.green);
    let color_blue = perc_to_u8(state.blue);
    let color_alpha = perc_to_u8(state.alpha);

    let final_color = Color::from_rgba8(color_red, color_green, color_blue, color_alpha);

    // Main layout is a horizontal flexbox with controls on the left and preview on the right.
    flex_row((
        // Controls column
        flex_col((
            label("Color Picker").text_size(24.0),
            control_slider("Red", state.red, color_red, |state, val| {
                state.red = val;
            }),
            control_slider("Green", state.green, color_green, |state, val| {
                state.green = val;
            }),
            control_slider("Blue", state.blue, color_blue, |state, val| {
                state.blue = val;
            }),
            flex_row((
                label("Alpha").width(40.px()),
                slider(0.0, 100.0, state.alpha, |state: &mut AppState, val| {
                    state.alpha = val;
                })
                .step(5.0)
                .disabled(!state.use_transparency)
                .width(200.px())
                .prop(BarColor(Color::from_rgb8(0x78, 0x71, 0x6c)))
                .prop(ThumbColor(Color::WHITE))
                .prop(ThumbRadius(10.0)),
                label(format!("{:.0}% [{}]", state.alpha, color_alpha)).width(60.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(10.0.px()),
            flex(
                Axis::Horizontal,
                (
                    checkbox(
                        "Transparency",
                        state.use_transparency,
                        |state: &mut AppState, checked| {
                            state.use_transparency = checked;
                            if checked {
                                state.alpha = state.saved_alpha;
                            } else {
                                state.saved_alpha = state.alpha;
                                state.alpha = 100.0;
                            }
                        },
                    ),
                    text_button("Reset", |state: &mut AppState| state.reset()),
                ),
            )
            .main_axis_alignment(MainAxisAlignment::Center)
            .gap(20.0.px()),
            FlexSpacer::Flex(1.0),
        ))
        .gap(15.0.px()),
        FlexSpacer::Fixed(10.px()),
        // Color preview box
        sized_box(
            // An empty label to create a view with a background.
            label(""),
        )
        .dims(Dim::Stretch)
        .background(Background::Color(final_color))
        .corner_radius(8.0)
        .flex(1.0),
    ))
    .gap(20.0.px())
    .padding(20.0)
}

// --- Application Entry Point ---

fn main() -> Result<(), EventLoopError> {
    let app_data = AppState::default();
    let min_window_size = LogicalSize::new(440., 300.);
    let window_options =
        WindowOptions::new("Slider Demo - Color Picker").with_min_inner_size(min_window_size);
    let app = Xilem::new_simple(app_data, app_logic, window_options);
    app.run_in(EventLoop::with_user_event())
}
