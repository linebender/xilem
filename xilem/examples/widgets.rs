// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget gallery for xilem/masonry

use masonry::dpi::LogicalSize;
use masonry::event_loop_runner::{EventLoop, EventLoopBuilder};
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::view::{button, checkbox, flex, label, progress_bar, FlexSpacer};
use xilem::{WidgetView, Xilem};

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct WidgetGallery {
    progress: Option<f32>,
}

fn app_logic(data: &mut WidgetGallery) -> impl WidgetView<WidgetGallery> {
    flex((
        label("this 'widgets' example currently only has 1 widget"),
        FlexSpacer::Flex(1.),
        progress_bar(data.progress),
        checkbox(
            "set indeterminate progress",
            data.progress.is_none(),
            |state: &mut WidgetGallery, checked| {
                if checked {
                    state.progress = None;
                } else {
                    state.progress = Some(0.5);
                }
            },
        ),
        button("change progress", |state: &mut WidgetGallery| {
            match state.progress {
                Some(ref mut v) => *v = (*v + 0.1).rem_euclid(1.),
                None => state.progress = Some(0.5),
            }
        }),
        FlexSpacer::Flex(1.),
    ))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = WidgetGallery {
        progress: Some(0.5),
    };

    let app = Xilem::new(data, app_logic);
    let min_window_size = LogicalSize::new(300., 200.);
    let window_size = LogicalSize::new(450., 300.);
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
