// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::{dpi::LogicalSize, error::EventLoopError, window::Window};
use xilem::{view::flex, EventLoop, EventLoopBuilder, WidgetView, Xilem};

struct HttpCats;

impl HttpCats {
    fn view(&mut self) -> impl WidgetView<Self> {
        flex(())
    }
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = HttpCats {};

    let app = Xilem::new(data, HttpCats::view);
    let min_window_size = LogicalSize::new(200., 200.);

    let window_attributes = Window::default_attributes()
        .with_title("HTTP cats")
        .with_resizable(true)
        .with_min_inner_size(min_window_size);

    app.run_windowed_in(event_loop, window_attributes)
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
