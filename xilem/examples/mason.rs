// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A playground used in the development for new Xilem Masonry features.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use std::time::Duration;

use masonry::properties::types::Length;
use winit::error::EventLoopError;
use xilem::core::{Resource, fork, provides, run_once, with_context, without_elements};
use xilem::style::Style as _;
use xilem::tokio::time;
use xilem::view::{
    Axis, FlexExt as _, FlexSpacer, PointerButton, button, button_any_pointer, checkbox, flex,
    flex_row, label, prose, task, text_input,
};
use xilem::{
    EventLoop, EventLoopBuilder, FontWeight, InsertNewline, TextAlign, WidgetView, WindowOptions,
    Xilem, palette,
};

const LOREM: &str = r"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi cursus mi sed euismod euismod. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Nullam placerat efficitur tellus at semper. Morbi ac risus magna. Donec ut cursus ex. Etiam quis posuere tellus. Mauris posuere dui et turpis mollis, vitae luctus tellus consectetur. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur eu facilisis nisl.

Phasellus in viverra dolor, vitae facilisis est. Maecenas malesuada massa vel ultricies feugiat. Vivamus venenatis et nibh nec pharetra. Phasellus vestibulum elit enim, nec scelerisque orci faucibus id. Vivamus consequat purus sit amet orci egestas, non iaculis massa porttitor. Vestibulum ut eros leo. In fermentum convallis magna in finibus. Donec justo leo, maximus ac laoreet id, volutpat ut elit. Mauris sed leo non neque laoreet faucibus. Aliquam orci arcu, faucibus in molestie eget, ornare non dui. Donec volutpat nulla in fringilla elementum. Aliquam vitae ante egestas ligula tempus vestibulum sit amet sed ante. ";

#[derive(Debug)]
struct SomeContext(u32);
impl Resource for SomeContext {}

/// A test for using resources.
///
/// Requires the `SomeContext` resource to be [provided](provides).
fn env_using() -> impl WidgetView<AppData> + use<> {
    with_context(|context: &mut SomeContext, _: &mut AppData| {
        button(format!("Context: {}", context.0), |_: &mut AppData| {
            tracing::warn!("Does nothing");
        })
    })
}

fn app_logic(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    // here's some logic, deriving state for the view from our state
    let count = data.count;
    let button_label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };

    // The actual UI Code starts here

    let axis = if data.active {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };

    let flex_sequence = (0..count)
        .map(|x| {
            (
                button(format!("+{x}"), move |data: &mut AppData| data.count += x),
                if data.active {
                    FlexSpacer::Flex(x as f64)
                } else {
                    FlexSpacer::Fixed(Length::px((count - x) as f64))
                },
            )
        })
        .collect::<Vec<_>>();

    let fizz_buzz_flex_sequence = [(3, "Fizz"), (5, "Buzz")].map(|c| {
        if data.count.abs() % c.0 == 0 {
            button(c.1, move |data: &mut AppData| {
                data.count += 1;
            })
            .into_any_flex()
        } else {
            FlexSpacer::Fixed(Length::px(10.0 * c.0 as f64)).into_any_flex()
        }
    });

    provides(
        |_: &mut AppData| SomeContext(120),
        fork(
            flex((
                env_using(),
                flex_row((
                    label("Label").color(palette::css::REBECCA_PURPLE),
                    label("Bold Label").weight(FontWeight::BOLD),
                    // TODO masonry doesn't allow setting disabled manually anymore?
                    // label("Disabled label").disabled(),
                )),
                flex(
                    text_input(
                        data.text_input_contents.clone(),
                        |data: &mut AppData, new_value| {
                            data.text_input_contents = new_value;
                        },
                    )
                    .insert_newline(InsertNewline::OnEnter),
                )
                // Manually adding a direction is equivalent to using flex_row
                .direction(Axis::Horizontal),
                prose(LOREM)
                    .text_alignment(TextAlign::Center)
                    .text_size(18.),
                button_any_pointer(button_label, |data: &mut AppData, button| match button {
                    None => {
                        // Usually this is a touch.
                    }
                    Some(PointerButton::Primary) => data.count += 1,
                    Some(PointerButton::Secondary) => data.count -= 1,
                    Some(PointerButton::Auxiliary) => data.count *= 2,
                    _ => (),
                }),
                checkbox("Check me", data.active, |data: &mut AppData, checked| {
                    data.active = checked;
                }),
                toggleable(data),
                env_using(),
                button("Decrement", |data: &mut AppData| data.count -= 1),
                button("Reset", |data: &mut AppData| data.count = 0),
                flex((fizz_buzz_flex_sequence, flex_sequence)).direction(axis),
            ))
            .padding(8.0),
            // The following `task` view only exists whilst the example is in the "active" state, so
            // the updates it performs will only be running whilst we are in that state.
            data.active.then(|| {
                task(
                    |proxy| async move {
                        let mut interval = time::interval(Duration::from_secs(1));
                        loop {
                            interval.tick().await;
                            let Ok(()) = proxy.message(()) else {
                                break;
                            };
                        }
                    },
                    |data: &mut AppData, ()| {
                        data.count += 1;
                    },
                )
            }),
        ),
    )
}

fn toggleable(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    if data.active {
        provides(
            |_| SomeContext(777),
            flex_row((
                button("Deactivate", |data: &mut AppData| {
                    data.active = false;
                }),
                button("Unlimited Power", |data: &mut AppData| {
                    data.count = -1_000_000;
                }),
                without_elements(run_once(|| {
                    tracing::warn!("The pathway to unlimited power has been revealed");
                })),
                env_using(),
            )),
        )
        .boxed()
    } else {
        button("Activate", |data: &mut AppData| data.active = true).boxed()
    }
}

struct AppData {
    text_input_contents: String,
    count: i32,
    active: bool,
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = AppData {
        count: 0,
        text_input_contents: "Not quite a placeholder".into(),
        active: false,
    };

    Xilem::new_simple(data, app_logic, WindowOptions::new("mason")).run_in(event_loop)
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
