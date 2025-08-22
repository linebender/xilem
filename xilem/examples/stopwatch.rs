// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A stopwatch to display elapsed time.

use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};

use masonry::dpi::LogicalSize;
use masonry::properties::types::{AsUnit, CrossAxisAlignment, MainAxisAlignment};
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use tokio::time;
use tracing::warn;
use winit::error::EventLoopError;
use xilem::core::fork;
use xilem::core::one_of::Either;
use xilem::view::{FlexSequence, FlexSpacer, button, flex, flex_row, label, task};
use xilem::{WidgetView, WindowOptions, Xilem};

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct Stopwatch {
    active: bool,
    /// The duration to add to the duration since the last instant.
    /// This is needed since you can pause a timer, and we need to account for all
    /// time the timer was active before the last start.
    added_duration: Duration,
    /// The absolute time of the last start for calculating elapsed time.
    last_start_time: Option<SystemTime>,
    /// The duration displayed; updated by by `update_display()`
    displayed_duration: Duration,
    /// An error string to display if there is an error.
    displayed_error: String,
    /// A list of the length of all completed splits. Does not include the current split.
    completed_lap_splits: Vec<Duration>,
    /// The duration of the main timer when the split was started.
    split_start_time: Duration,
}

impl Stopwatch {
    fn start(&mut self) {
        self.last_start_time = Some(SystemTime::now());
        self.active = true;
        self.update_display();
    }

    fn stop(&mut self) {
        let dur_since_last_instant = self
            .last_start_time
            .expect("stop should only be called when the start time is set")
            .elapsed();
        match dur_since_last_instant {
            Ok(dur) => {
                self.added_duration = self.added_duration.add(dur);
                self.displayed_error = "".into();
            }
            Err(err) => {
                self.displayed_error = format!("failed to calculate elapsed time: {err}");
            }
        }
        self.last_start_time = None;
        self.active = false;
        self.update_display();
    }

    fn reset(&mut self) {
        self.active = false;
        self.last_start_time = None;
        self.added_duration = Duration::ZERO;
        self.completed_lap_splits.clear();
        self.split_start_time = Duration::ZERO;
        self.update_display();
    }

    fn lap(&mut self) {
        let split_end = self.get_current_duration();
        self.completed_lap_splits
            .push(split_end.sub(self.split_start_time));
        self.split_start_time = split_end;
    }

    fn get_current_duration(&self) -> Duration {
        match self.last_start_time {
            Some(last_start_time) => match last_start_time.elapsed() {
                Ok(elapsed) => self.added_duration + elapsed,
                Err(err) => {
                    warn!("error calculating elapsed time: {}", err.to_string());
                    self.added_duration
                }
            },
            _ => self.added_duration,
        }
    }

    fn update_display(&mut self) {
        self.displayed_duration = self.get_current_duration();
    }
}

fn get_formatted_duration(dur: Duration) -> String {
    let seconds = dur.as_secs_f64() % 60.0;
    let minutes = (dur.as_secs() / 60) % 60;
    let hours = (dur.as_secs() / 60) / 60;
    format!("{hours}:{minutes:0>2}:{seconds:0>4.1}")
}

fn app_logic(data: &mut Stopwatch) -> impl WidgetView<Stopwatch> + use<> {
    fork(
        flex((
            FlexSpacer::Fixed(5.px()),
            label(get_formatted_duration(data.displayed_duration)).text_size(70.0),
            flex_row((lap_reset_button(data), start_stop_button(data))),
            FlexSpacer::Fixed(1.px()),
            laps_section(data),
            label(data.displayed_error.as_ref()),
        )),
        data.active.then(|| {
            // Only update while active.
            task(
                |proxy| async move {
                    let mut interval = time::interval(Duration::from_millis(50));
                    loop {
                        interval.tick().await;
                        let Ok(()) = proxy.message(()) else {
                            break;
                        };
                    }
                },
                |data: &mut Stopwatch, ()| {
                    data.update_display();
                },
            )
        }),
    )
}

/// Creates a list of items that shows the lap number, split time, and total cumulative time.
fn laps_section(data: &mut Stopwatch) -> impl FlexSequence<Stopwatch> + use<> {
    let mut items = Vec::new();
    let mut total_dur = Duration::ZERO;
    let current_lap = data.completed_lap_splits.len();
    for (i, split_dur) in data.completed_lap_splits.iter().enumerate() {
        total_dur = total_dur.add(*split_dur);
        items.push(single_lap(i, *split_dur, total_dur));
    }
    let current_split_duration = data.get_current_duration().sub(total_dur);
    // Add the current lap, which is not stored in completed_lap_splits
    items.push(single_lap(
        current_lap,
        current_split_duration,
        data.get_current_duration(),
    ));
    items.reverse();
    items
}

fn single_lap(
    lap_id: usize,
    split_dur: Duration,
    total_dur: Duration,
) -> impl WidgetView<Stopwatch> {
    flex_row((
        FlexSpacer::Flex(1.0),
        label(format!("Lap {}", lap_id + 1)),
        label(get_formatted_duration(split_dur)),
        label(get_formatted_duration(total_dur)),
        FlexSpacer::Flex(1.0),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Start)
    .must_fill_major_axis(true)
}

fn start_stop_button(data: &mut Stopwatch) -> impl WidgetView<Stopwatch> + use<> {
    if data.active {
        Either::A(button("Stop", |data: &mut Stopwatch| {
            data.stop();
        }))
    } else {
        Either::B(button("Start", |data: &mut Stopwatch| {
            data.start();
        }))
    }
}

fn lap_reset_button(data: &mut Stopwatch) -> impl WidgetView<Stopwatch> + use<> {
    if data.active {
        Either::A(button("  Lap  ", |data: &mut Stopwatch| {
            data.lap();
        }))
    } else {
        Either::B(button("Reset", |data: &mut Stopwatch| {
            data.reset();
        }))
    }
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let mut data = Stopwatch {
        active: false,
        added_duration: Duration::ZERO,
        last_start_time: None,
        displayed_duration: Duration::ZERO,
        displayed_error: "".into(),
        completed_lap_splits: Vec::new(),
        split_start_time: Duration::ZERO,
    };
    data.update_display();

    let window_options = WindowOptions::new("Stopwatch")
        .with_min_inner_size(LogicalSize::new(300., 200.))
        .with_initial_inner_size(LogicalSize::new(450., 300.));
    let app = Xilem::new_simple(data, app_logic, window_options);
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
