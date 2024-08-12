//! This example uses variable fonts in a touch sensitive digital clock.

use masonry::parley::fontique::Weight;
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, variable_label, CrossAxisAlignment, FlexExt, FlexSpacer},
    EventLoop, EventLoopBuilder, WidgetView, Xilem,
};

const LOREM: &str = r"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi cursus mi sed euismod euismod. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Nullam placerat efficitur tellus at semper. Morbi ac risus magna. Donec ut cursus ex. Etiam quis posuere tellus. Mauris posuere dui et turpis mollis, vitae luctus tellus consectetur. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur eu facilisis nisl.

Phasellus in viverra dolor, vitae facilisis est. Maecenas malesuada massa vel ultricies feugiat. Vivamus venenatis et nibh nec pharetra. Phasellus vestibulum elit enim, nec scelerisque orci faucibus id. Vivamus consequat purus sit amet orci egestas, non iaculis massa porttitor. Vestibulum ut eros leo. In fermentum convallis magna in finibus. Donec justo leo, maximus ac laoreet id, volutpat ut elit. Mauris sed leo non neque laoreet faucibus. Aliquam orci arcu, faucibus in molestie eget, ornare non dui. Donec volutpat nulla in fringilla elementum. Aliquam vitae ante egestas ligula tempus vestibulum sit amet sed ante. ";

struct Clocks {
    weight: f32,
}

fn app_logic(data: &mut Clocks) -> impl WidgetView<Clocks> {
    flex((
        FlexSpacer::Fixed(40.),
        button("Increase", |data: &mut Clocks| {
            data.weight = (data.weight + 100.).clamp(1., 1000.);
        }),
        button("Decrease", |data: &mut Clocks| {
            data.weight = (data.weight - 100.).clamp(1., 1000.);
        }),
        variable_label(LOREM)
            .text_size(36.)
            .target_weight(data.weight, 400.)
            .flex(CrossAxisAlignment::Start),
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
