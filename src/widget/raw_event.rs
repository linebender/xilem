// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Handling of platform and integration events at the widget level.
//!
//! Note: arguably this module should be renamed, perhaps we should use
//! "event" for this level and maybe "message" at the View level.
use std::collections::HashSet;

use vello::kurbo::{Point, Rect, Vec2};
use winit::event::{Modifiers, MouseButton};

#[derive(Debug, Clone)]
pub enum Event {
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMove(MouseEvent),
    MouseWheel(MouseEvent),
    MouseLeft(),
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// The position of the mouse in the coordinate space of the receiver.
    pub pos: Point,
    /// The position of the mouse in the window coordinate space.
    pub window_pos: Point,
    pub buttons: HashSet<MouseButton>,
    pub mods: Modifiers,
    pub count: u8,
    pub focus: bool,
    pub button: Option<MouseButton>,
    pub wheel_delta: Option<ScrollDelta>,
}

#[derive(Debug, Clone)]
pub enum ScrollDelta {
    Precise(Vec2),
    Lines(isize, isize),
}

#[derive(Debug)]
pub enum LifeCycle {
    HotChanged(bool),
    ViewContextChanged(ViewContext),
    TreeUpdate,
}

#[derive(Debug)]
pub struct ViewContext {
    pub window_origin: Point,
    pub clip: Rect,
    pub mouse_position: Option<Point>,
}

impl Default for MouseEvent {
    fn default() -> Self {
        MouseEvent {
            pos: Point::ZERO,
            window_pos: Point::ZERO,
            buttons: HashSet::<MouseButton>::new(),
            mods: Modifiers::default(),
            count: 0,
            focus: false,
            button: None,
            wheel_delta: None,
        }
    }
}

impl ViewContext {
    pub fn translate_to(&self, new_origin: Point) -> ViewContext {
        let translate = new_origin.to_vec2();
        ViewContext {
            window_origin: self.window_origin + translate,
            clip: self.clip - translate,
            mouse_position: self.mouse_position.map(|p| p - translate),
        }
    }
}

/// Crush all pointer events into a single pointer that counts clicks
/// and attaches positions to events that don't contain them.
#[derive(Default)]
pub struct PointerCrusher {
    e: MouseEvent,
    counter: ClickCounter,
}

impl PointerCrusher {
    pub fn new() -> Self {
        PointerCrusher::default()
    }

    pub fn mods(&mut self, mods: Modifiers) {
        self.e.mods = mods;
    }

    pub fn pressed(&mut self, button: MouseButton) -> MouseEvent {
        self.e.wheel_delta = None;
        self.e.buttons.insert(button);
        self.e.count = self.counter.count_for_click(self.e.pos);
        self.e.button = Some(button);
        self.e.clone()
    }

    pub fn released(&mut self, button: MouseButton) -> MouseEvent {
        self.e.wheel_delta = None;
        self.e.buttons.remove(&button);
        self.e.button = Some(button);
        self.e.clone()
    }

    pub fn moved(&mut self, pos: Point) -> MouseEvent {
        self.e.wheel_delta = None;
        self.e.button = None;
        self.e.pos = pos;
        self.e.window_pos = pos;
        self.e.clone()
    }

    pub fn wheel(&mut self, wheel_delta: ScrollDelta) -> MouseEvent {
        self.e.wheel_delta = Some(wheel_delta);
        self.e.button = None;
        self.e.clone()
    }
}

use instant::Instant;
use std::cell::Cell;
use std::time::Duration;

// This is the default timing on windows.
const MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
// the max distance between two clicks for them to count as a multi-click
const MULTI_CLICK_MAX_DISTANCE: f64 = 5.0;

/// A small helper for determining the click-count of a mouse-down event.
///
/// Click-count is incremented if both the duration and distance between a pair
/// of clicks are below some threshold.
#[derive(Debug, Clone)]
struct ClickCounter {
    max_interval: Cell<Duration>,
    max_distance: Cell<f64>,
    last_click: Cell<Instant>,
    last_pos: Cell<Point>,
    click_count: Cell<u8>,
}

#[allow(dead_code)]
impl ClickCounter {
    /// Create a new `ClickCounter` with the given interval and distance.
    pub fn new(max_interval: Duration, max_distance: f64) -> ClickCounter {
        ClickCounter {
            max_interval: Cell::new(max_interval),
            max_distance: Cell::new(max_distance),
            last_click: Cell::new(Instant::now()),
            click_count: Cell::new(0),
            last_pos: Cell::new(Point::new(f64::MAX, 0.0)),
        }
    }

    pub fn set_interval_ms(&self, millis: u64) {
        self.max_interval.set(Duration::from_millis(millis));
    }

    pub fn set_distance(&self, distance: f64) {
        self.max_distance.set(distance);
    }

    /// Return the click count for a click occurring now, at the provided position.
    pub fn count_for_click(&self, click_pos: Point) -> u8 {
        let click_time = Instant::now();
        let last_time = self.last_click.replace(click_time);
        let last_pos = self.last_pos.replace(click_pos);
        let elapsed = click_time - last_time;
        let distance = last_pos.distance(click_pos);
        if elapsed > self.max_interval.get() || distance > self.max_distance.get() {
            self.click_count.set(0);
        }
        let click_count = self.click_count.get().saturating_add(1);
        self.click_count.set(click_count);
        click_count
    }
}

impl Default for ClickCounter {
    fn default() -> Self {
        ClickCounter::new(MULTI_CLICK_INTERVAL, MULTI_CLICK_MAX_DISTANCE)
    }
}
