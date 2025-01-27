// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget gallery for xilem/masonry
#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use masonry::app::{EventLoop, EventLoopBuilder};
use masonry::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::adapt;
use xilem::view::{
    button, checkbox, flex, flex_item, progress_bar, sized_box, slider, Axis, FlexSpacer,
};
use xilem::{Color, WidgetView, Xilem};

const SPACER_WIDTH: f64 = 10.;

#[derive(Clone, Copy, PartialEq)]
enum SliderDirection {
    Horizontal,
    Vertical,
}

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct WidgetGallery {
    progress: Option<f64>,
    checked: bool,
    slider_value: f64,
    slider_direction: SliderDirection,
}

fn progress_bar_view(data: Option<f64>) -> impl WidgetView<Option<f64>> {
    flex((
        progress_bar(data),
        checkbox(
            "set indeterminate progress",
            data.is_none(),
            |state: &mut Option<f64>, checked| {
                *state = if checked { None } else { Some(0.5) };
            },
        ),
        button("change progress", |state: &mut Option<f64>| match state {
            Some(v) => *v = (*v + 0.1).rem_euclid(1.),
            None => *state = Some(0.5),
        }),
    ))
}

fn checkbox_view(data: bool) -> impl WidgetView<bool> {
    checkbox("a simple checkbox", data, |data, new_state| {
        *data = new_state;
    })
}

fn slider_view(data: f64, direction: SliderDirection) -> impl WidgetView<f64> {
    let axis = match direction {
        SliderDirection::Horizontal => Axis::Horizontal,
        SliderDirection::Vertical => Axis::Vertical,
    };

    slider(0.0..1.0, data, |state, value| *state = value)
        .with_color(Color::from_rgb8(100, 150, 200))
        .with_track_color(Color::from_rgb8(200, 150, 100))
        .with_step(0.1)
        .direction(axis)
}

/// Wrap `inner` in a box with a border
fn border_box<State: 'static, Action: 'static>(
    inner: impl WidgetView<State, Action>,
) -> impl WidgetView<State, Action> {
    sized_box(
        flex((
            FlexSpacer::Flex(1.),
            flex((FlexSpacer::Flex(1.), inner, FlexSpacer::Flex(1.))),
            FlexSpacer::Flex(1.),
        ))
        .direction(Axis::Horizontal),
    )
    .border(Color::WHITE, 2.)
    .width(450.)
    .height(200.)
}

/// Top-level view
fn app_logic(data: &mut WidgetGallery) -> impl WidgetView<WidgetGallery> {
    // Use a `sized_box` to pad the window contents
    sized_box(
        flex((
            adapt(
                flex_item(border_box(progress_bar_view(data.progress)), 1.),
                |data: &mut WidgetGallery, thunk| thunk.call(&mut data.progress),
            ),
            adapt(
                flex_item(border_box(checkbox_view(data.checked)), 1.),
                |data: &mut WidgetGallery, thunk| thunk.call(&mut data.checked),
            ),
            flex_item(
                sized_box(border_box(
                    flex((
                        adapt(
                            sized_box(button(
                                match data.slider_direction {
                                    SliderDirection::Horizontal => "Switch to Vertical",
                                    SliderDirection::Vertical => "Switch to Horizontal",
                                },
                                |state: &mut WidgetGallery| {
                                    state.slider_direction = match state.slider_direction {
                                        SliderDirection::Horizontal => SliderDirection::Vertical,
                                        SliderDirection::Vertical => SliderDirection::Horizontal,
                                    };
                                },
                            ))
                            .width(200.0)
                            .height(40.0),
                            |data: &mut WidgetGallery, thunk| thunk.call(data),
                        ),
                        adapt(
                            sized_box(slider_view(data.slider_value, data.slider_direction))
                                .width(match data.slider_direction {
                                    SliderDirection::Horizontal => 300.0,
                                    SliderDirection::Vertical => 100.0,
                                })
                                .height(match data.slider_direction {
                                    SliderDirection::Horizontal => 100.0,
                                    SliderDirection::Vertical => 300.0,
                                }),
                            |data: &mut WidgetGallery, thunk| thunk.call(&mut data.slider_value),
                        ),
                    ))
                    .direction(Axis::Vertical)
                    .gap(20.0),
                ))
                .height(250.0)
                .width(450.0),
                1.,
            ),
        ))
        .gap(SPACER_WIDTH)
        .direction(Axis::Horizontal),
    )
    .padding(20.0)
    .border(Color::TRANSPARENT, SPACER_WIDTH)
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = WidgetGallery {
        progress: Some(0.5),
        checked: false,
        slider_value: 0.5,
        slider_direction: SliderDirection::Horizontal,
    };

    let app = Xilem::new(data, app_logic);
    let min_window_size = LogicalSize::new(300., 200.);
    let window_size = LogicalSize::new(650., 500.);
    let window_attributes = Window::default_attributes()
        .with_title("Xilem Widgets")
        .with_resizable(true)
        .with_min_inner_size(min_window_size)
        .with_inner_size(window_size);
    app.run_windowed_in(event_loop, window_attributes)?;
    Ok(())
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
