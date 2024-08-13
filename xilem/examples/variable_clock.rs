//! This example uses variable fonts in a touch sensitive digital clock.

use masonry::parley::{
    fontique::Weight,
    style::{FontFamily, FontStack},
};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, variable_label, Axis, CrossAxisAlignment, FlexExt, FlexSpacer},
    EventLoop, EventLoopBuilder, WidgetView, Xilem,
};

// TODO: Move to a more full-featured (e.g. multiple time-zones) example.
/// The text used in the example. This will be replaced.
/// Notice that not all of the text is included in the font subset chosen.
/// This is an intentional choice to show the graceful fallback of animated weight still working.
const LOREM: &str = r"Office hours is at 16:00";

/// The state of the application, owned by Xilem and updated by the callbacks below.
struct Clocks {
    /// The font [weight](Weight) used for the values.
    weight: f32,
}

fn app_logic(data: &mut Clocks) -> impl WidgetView<Clocks> {
    flex((
        // HACK: We add a spacer at the top for Android. See https://github.com/rust-windowing/winit/issues/2308
        FlexSpacer::Fixed(40.),
        flex((
            button("Increase", |data: &mut Clocks| {
                data.weight = (data.weight + 100.).clamp(1., 1000.);
            }),
            button("Decrease", |data: &mut Clocks| {
                data.weight = (data.weight - 100.).clamp(1., 1000.);
            }),
            button("Minimum", |data: &mut Clocks| {
                data.weight = 1.;
            }),
            button("Maximum", |data: &mut Clocks| {
                data.weight = 1000.;
            }),
        ))
        .direction(Axis::Horizontal),
        variable_label(LOREM)
            .text_size(36.)
            // Use the roboto flex we have just loaded.
            .with_font(FontStack::List(&[FontFamily::Named("Roboto Flex")]))
            // This is the key functionality
            .target_weight(data.weight, 400.)
            .flex(CrossAxisAlignment::Start),
    ))
}

/// A subset of [Roboto Flex](https://fonts.google.com/specimen/Roboto+Flex), used under the OFL.
/// This is a variable font, and so can be.
/// The version in the repository supports the numbers 0-9 and `:`, as it is included for this example,
/// which is using it for clocks.
// TODO: Double check which subset we want to commit.
/// Full details can also be found in `xilem/resources/fonts/roboto_flex/README` from
/// the workspace root.
const ROBOTO_FLEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/fonts/roboto_flex/",
    // The full font file is *not* included in this repository, due to size constraints.
    // If you download the full font, you can use it by moving it into the roboto_flex folder,
    // then swapping which of the following two lines is commented out:
    // "RobotoFlex-VariableFont_GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght.ttf",
    "RobotoFlex-Subset.ttf"
));

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Clocks {
        weight: Weight::BLACK.value(),
    };

    // Load Roboto Flex so that it can be used at runtime.
    let app = Xilem::new(data, app_logic).with_font(ROBOTO_FLEX);

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
