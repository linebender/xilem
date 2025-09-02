// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing the capabilities of the Slider widget.
//!
//! This demo presents a simple "Color Picker" control panel, where each color
//! channel (Red, Green, Blue, Alpha) can be adjusted using a slider.
//! The result is displayed in a preview box, demonstrating how widgets can
//! react to and manipulate a shared application state.

use masonry::peniko::Color;
use masonry::properties::types::{AsUnit, CrossAxisAlignment};
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::{
    EventLoop, WidgetView, WindowOptions, Xilem,
    view::{Axis, button, checkbox, flex, label, sized_box, slider},
};

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
    is_opaque: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            red: 95.0,
            green: 10.0,
            blue: 60.0,
            alpha: 100.0,
            saved_alpha: 100.0,
            is_opaque: false,
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
) -> impl WidgetView<AppState>
where
    F: Fn(&mut AppState, f64) + Send + Sync + 'static,
{
    flex(
        Axis::Horizontal,
        (
            label(label_text),
            slider(0.0, 100.0, value, on_change),
            label(format!("{:.0}% [{}]", value, u_value)),
        ),
    )
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(10.0.px())
}

// --- Main UI Logic ---

/// The main logic for building the user interface.
fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    // Convert the 0-100 state values to 0-255 color channel values.
    #[allow(clippy::cast_possible_truncation, reason = "It's OK")]
    let color_red = (state.red * 2.56).clamp(0.0, 255.0) as u8;
    #[allow(clippy::cast_possible_truncation, reason = "It's OK")]
    let color_green = (state.green * 2.56).clamp(0.0, 255.0) as u8;
    #[allow(clippy::cast_possible_truncation, reason = "It's OK")]
    let color_blue = (state.blue * 2.56).clamp(0.0, 255.0) as u8;
    #[allow(clippy::cast_possible_truncation, reason = "It's OK")]
    let color_alpha = (state.alpha * 2.56).clamp(0.0, 255.0) as u8;

    let final_color = Color::from_rgba8(color_red, color_green, color_blue, color_alpha);

    // Main layout is a horizontal flexbox with controls on the left and preview on the right.
    flex(
        Axis::Horizontal,
        (
            // Controls column
            flex(
                Axis::Vertical,
                (
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
                    flex(
                        Axis::Horizontal,
                        (
                            label("Alpha"),
                            slider(0.0, 100.0, state.alpha, |state: &mut AppState, val| {
                                state.alpha = val;
                            })
                            .step(5.0)
                            .disabled(state.is_opaque)
                            .active_track_color(Color::from_rgb8(0x78, 0x71, 0x6c))
                            .thumb_color(Color::WHITE)
                            .thumb_radius(10.0),
                            label(format!("{:.0}% [{}]", state.alpha, color_alpha)),
                        ),
                    )
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(10.0.px()),
                    flex(
                        Axis::Horizontal,
                        (
                            checkbox(
                                "Opaque",
                                state.is_opaque,
                                |state: &mut AppState, checked| {
                                    state.is_opaque = checked;
                                    if checked {
                                        state.saved_alpha = state.alpha;
                                        state.alpha = 100.0;
                                    } else {
                                        state.alpha = state.saved_alpha;
                                    }
                                },
                            ),
                            button("Reset", |state: &mut AppState| state.reset()),
                        ),
                    )
                    .gap(20.0.px()),
                ),
            )
            .gap(15.0.px()),
            // Color preview box
            sized_box(
                // An empty label to create a view with a background.
                label(""),
            )
            .expand()
            .background(masonry::properties::Background::Color(final_color))
            .corner_radius(8.0),
        ),
    )
    .gap(20.0.px())
    .padding(20.0)
}

// --- Application Entry Point ---

fn main() -> Result<(), EventLoopError> {
    let data = AppState::default();
    let window_options = WindowOptions::new("Slider Demo - Color Picker");
    let app = Xilem::new_simple(data, app_logic, window_options);
    app.run_in(EventLoop::with_user_event())
}
