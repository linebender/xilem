//! This example uses variable fonts in a touch sensitive digital clock.

use masonry::parley::fontique::Weight;
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, variable_label},
    EventLoop, EventLoopBuilder, WidgetView, Xilem,
};

struct Clocks {
    weight: f32,
}

fn app_logic(data: &mut Clocks) -> impl WidgetView<Clocks> {
    flex((
        variable_label("Text")
            .text_size(72.)
            .target_weight(data.weight, 400.),
        button("Increase", |data: &mut Clocks| {
            data.weight = (data.weight + 100.).clamp(1., 1000.);
        }),
        button("Decrease", |data: &mut Clocks| {
            data.weight = (data.weight - 100.).clamp(1., 1000.);
        }),
    ))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Clocks {
        weight: Weight::BLACK.value(),
    };

    let app = Xilem::new(data, app_logic);

    app.run_windowed(event_loop, "Clocks".into())?;
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
