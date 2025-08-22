// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget gallery for xilem/masonry

use masonry::dpi::LogicalSize;
use masonry::properties::types::{AsUnit, Length};
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use winit::error::EventLoopError;
use xilem::style::Style as _;
use xilem::view::{
    FlexSpacer, button, checkbox, flex, flex_row, indexed_stack, progress_bar, sized_box,
};
use xilem::{Color, WidgetView, WindowOptions, Xilem};
use xilem_core::lens;

const SPACER_WIDTH: Length = Length::const_px(10.);

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct WidgetGallery {
    tab: GalleryTab,
    progress: Option<f64>,
    checked: bool,
}

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum GalleryTab {
    Progress = 0,
    Checkbox,
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

/// Wrap `inner` in a box with a border
fn border_box<State: 'static, Action: 'static>(
    inner: impl WidgetView<State, Action>,
) -> impl WidgetView<State, Action> {
    sized_box(flex_row((
        FlexSpacer::Flex(1.),
        flex((FlexSpacer::Flex(1.), inner, FlexSpacer::Flex(1.))),
        FlexSpacer::Flex(1.),
    )))
    .border(Color::WHITE, 2.)
    .width(450.px())
    .height(200.px())
}

/// Top-level view
fn app_logic(data: &mut WidgetGallery) -> impl WidgetView<WidgetGallery> + use<> {
    // Use a `sized_box` to pad the window contents
    sized_box(
        flex((
            flex_row((
                button("Progress", |data: &mut WidgetGallery| {
                    data.tab = GalleryTab::Progress;
                })
                .disabled(data.tab == GalleryTab::Progress),
                button("Checkbox", |data: &mut WidgetGallery| {
                    data.tab = GalleryTab::Checkbox;
                })
                .disabled(data.tab == GalleryTab::Checkbox),
            )),
            indexed_stack((
                lens(
                    |progress| border_box(progress_bar_view(*progress)),
                    |data: &mut WidgetGallery| &mut data.progress,
                ),
                lens(
                    |checked| border_box(checkbox_view(*checked)),
                    |data: &mut WidgetGallery| &mut data.checked,
                ),
            ))
            .active(data.tab as usize),
        ))
        .gap(SPACER_WIDTH),
    )
    .padding(SPACER_WIDTH.get())
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    // Set up the initial state of the app
    let data = WidgetGallery {
        tab: GalleryTab::Progress,
        progress: Some(0.5),
        checked: false,
    };

    // Instantiate and run the UI using the passed event loop.
    let min_window_size = LogicalSize::new(300., 200.);
    let window_size = LogicalSize::new(650., 500.);
    let app = Xilem::new_simple(
        data,
        app_logic,
        WindowOptions::new("Xilem widgets")
            .with_min_inner_size(min_window_size)
            .with_initial_inner_size(window_size),
    );
    app.run_in(event_loop)?;
    Ok(())
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::builder())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
