// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget gallery for xilem/masonry

use masonry::dpi::LogicalSize;
use masonry::event_loop_runner::{EventLoop, EventLoopBuilder};
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::adapt;
use xilem::view::{button, checkbox, flex, flex_item, progress_bar, sized_box, Axis, FlexSpacer};
use xilem::{Color, WidgetView, Xilem};

const SPACER_WIDTH: f64 = 10.;

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct WidgetGallery {
    progress: Option<f64>,
    checked: bool,
}

fn progress_bar_view(data: Option<f64>) -> impl WidgetView<Option<f64>> {
    flex((
        progress_bar(data),
        checkbox(
            "set indeterminate progress",
            data.is_none(),
            |state: &mut Option<f64>, checked| {
                if checked {
                    *state = None;
                } else {
                    *state = Some(0.5);
                }
            },
        ),
        button("change progress", |state: &mut Option<f64>| match state {
            Some(ref mut v) => *v = (*v + 0.1).rem_euclid(1.),
            None => *state = Some(0.5),
        }),
    ))
}

fn checkbox_view(data: bool) -> impl WidgetView<bool> {
    checkbox("a simple checkbox", data, |data, new_state| {
        *data = new_state;
    })
}

/// Wrap widgets in a box with a border
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
        ))
        .gap(SPACER_WIDTH)
        .direction(Axis::Horizontal),
    )
    .border(Color::TRANSPARENT, SPACER_WIDTH)
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    // Set up the initial state of the app
    let data = WidgetGallery {
        progress: Some(0.5),
        checked: false,
    };

    // Instantiate and run the UI using the passed event loop.
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

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

// Boilerplate code for android: Identical across all applications

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
// We believe that there are no other declarations using this name in the compiled objects here
#[allow(unsafe_code)]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}

// TODO: This is a hack because of how we handle our examples in Cargo.toml
// Ideally, we change Cargo to be more sensible here?
#[cfg(target_os = "android")]
#[allow(dead_code)]
fn main() {
    unreachable!()
}
