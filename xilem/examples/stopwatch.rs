// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A stopwatch to display elapsed time.

use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};
use tokio::time;
use winit::error::EventLoopError;
use winit::window::Window;
use masonry::dpi::LogicalSize;
use masonry::event_loop_runner::{EventLoop, EventLoopBuilder};
use masonry::widget::{Axis, CrossAxisAlignment, MainAxisAlignment};
use xilem::{WidgetView, Xilem};
use xilem::view::{AnyFlexChild, async_repeat, button, flex, FlexExt, FlexSpacer, label};
use tracing::warn;
use xilem_core::fork;

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct Stopwatch {
    active: bool,
    // The duration to add to the duration since the last instant.
    // This is needed since you can pause a timer, and we need to account for all
    // time the timer was active before the last start.
    added_duration: Duration,
    last_start_time: Option<SystemTime>,
    displayed_duration: Duration,
    displayed_error: String,
    completed_lap_splits: Vec<Duration>,
    split_start_time: Duration,
}

impl Stopwatch {
    fn start(&mut self) {
        self.last_start_time = Some(SystemTime::now());
        self.active = true;
        self.update_display();
    }

    fn stop(&mut self) {
        let dur_since_last_instant = self.last_start_time
            .expect("stop should only be called when the start time is set").elapsed();
        match dur_since_last_instant {
            Ok(dur) => {
                self.added_duration = self.added_duration.add(dur);
                self.displayed_error = "".into();
            },
            Err(err) => {
                self.displayed_error = format!("failed to calculate elapsed time: {}", err.to_string());
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
        self.completed_lap_splits.push(split_end.sub(self.split_start_time));
        self.split_start_time = split_end;
    }

    fn get_current_duration(&self) -> Duration {
        match self.last_start_time {
            Some(last_start_time) => {
                match last_start_time.elapsed() {
                    Ok(elapsed) => {
                        self.added_duration + elapsed
                    },
                    Err(err) => {
                        warn!("error calculating elapsed time: {}", err.to_string());
                        self.added_duration
                    }
                }
            }
            _ => {self.added_duration}
        }
    }

    fn update_display(&mut self) {
        self.displayed_duration = self.get_current_duration();
    }
}

fn get_formatted_duration(dur: &Duration) -> String {
    let seconds = dur.as_secs_f64() % 60.0;
    let minutes = (dur.as_secs() / 60) % 60;
    let hours = (dur.as_secs() / 60) / 60;
    format!("{hours}:{minutes:0>2}:{seconds:0>4.1}")
}

fn app_logic(data: &mut Stopwatch) -> impl WidgetView<Stopwatch> {
    fork(
        flex((
                 FlexSpacer::Fixed(5.0),
                 label(get_formatted_duration(&data.displayed_duration)).text_size(70.0),
                 flex((
                          lap_reset_button(data),
                          start_stop_button(data),
                      ),
                 ).direction(Axis::Horizontal),
                 laps_section(data),
                 label(data.displayed_error.as_ref()),
        ))
            .main_axis_alignment(MainAxisAlignment::Start)
            .cross_axis_alignment(CrossAxisAlignment::Center),
        async_repeat(
             |proxy| async move {
                 let mut interval = time::interval(Duration::from_millis(50));
                 loop {
                     interval.tick().await;
                     let Ok(()) = proxy.message(()) else {
                         break;
                     };
                 }
             },
             |data: &mut Stopwatch, ()|
             if data.active {
                 data.update_display();
             },
        )
    )
}

fn laps_section(data: &mut Stopwatch) -> Vec<AnyFlexChild<Stopwatch>> {
    let mut items: Vec<AnyFlexChild<Stopwatch>> = Vec::new();
    let mut total_dur = Duration::ZERO;
    let current_lap = data.completed_lap_splits.len();

    for (i, split_dur) in data.completed_lap_splits.iter().enumerate() {
        total_dur = total_dur.add(*split_dur);
        items.push(single_lap(i, split_dur, &total_dur).into_any_flex());
    }
    let current_split_duration = data.get_current_duration().sub(total_dur);
    let mut reversed  = Vec::new();
    reversed.push(single_lap(current_lap, &current_split_duration, &data.get_current_duration()).into_any_flex());
    for item in items.into_iter().rev() {
        reversed.push(item);
    }
    reversed
}

fn single_lap(lap_id: usize, split_dur: &Duration, total_dur: &Duration) -> impl WidgetView<Stopwatch> {
    flex((
        FlexSpacer::Flex(1.0),
        label(format!("Lap {}", lap_id + 1)).flex(1.0),
        label(get_formatted_duration(split_dur)).flex(1.0),
        label(get_formatted_duration(total_dur)).flex(1.0),
    ))
        .direction(Axis::Horizontal)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
}

fn start_stop_button(data: &mut Stopwatch) -> AnyFlexChild<Stopwatch> {
    return if data.active {
        button("Stop", |data: &mut Stopwatch| {
            data.stop();
        }).into_any_flex()
    } else {
        button("Start", |data: &mut Stopwatch| {
            data.start();
        }).into_any_flex()
    }
}

fn lap_reset_button(data: &mut Stopwatch) -> AnyFlexChild<Stopwatch> {
    return if data.active {
        button("  Lap  ", |data: &mut Stopwatch| {
            data.lap();
        }).into_any_flex()
    } else {
        button("Reset", |data: &mut Stopwatch| {
            data.reset();
        }).into_any_flex()
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

    let app = Xilem::new(data, app_logic);
    let min_window_size = LogicalSize::new(300., 200.);
    let window_size = LogicalSize::new(450., 300.);
    let window_attributes = Window::default_attributes()
        .with_title("Stopwatch")
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