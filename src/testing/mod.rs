// Copyright 2020 The Druid Authors.
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

//! Helper tools for writing unit tests.

#![cfg(not(tarpaulin_include))]

#[cfg(not(tarpaulin_include))]
mod harness;
#[cfg(not(tarpaulin_include))]
mod helper_widgets;
#[cfg(not(tarpaulin_include))]
mod mock_timer_queue;
#[cfg(not(tarpaulin_include))]
mod screenshots;
#[cfg(not(tarpaulin_include))]
mod snapshot_utils;

use druid_shell::{Modifiers, MouseButton, MouseButtons};
pub use harness::{TestHarness, HARNESS_DEFAULT_SIZE};
pub use helper_widgets::{
    ModularWidget, Record, Recorder, Recording, ReplaceChild, TestWidgetExt, REPLACE_CHILD,
};
pub(crate) use mock_timer_queue::MockTimerQueue;

use crate::kurbo::{Point, Vec2};
use crate::{MouseEvent, WidgetId};

/// Helper function to construct a "move to this position" mouse event.
pub fn mouse_move(p: impl Into<Point>) -> MouseEvent {
    let pos = p.into();
    MouseEvent {
        pos,
        window_pos: pos,
        buttons: MouseButtons::default(),
        mods: Modifiers::default(),
        count: 0,
        focus: false,
        button: MouseButton::None,
        wheel_delta: Vec2::ZERO,
    }
}

/// Helper function to construct a "scroll by n ticks" mouse event.
pub fn mouse_scroll(p: impl Into<Point>, delta: impl Into<Vec2>) -> MouseEvent {
    let pos = p.into();
    MouseEvent {
        pos,
        window_pos: pos,
        buttons: MouseButtons::default(),
        mods: Modifiers::default(),
        count: 0,
        focus: false,
        button: MouseButton::None,
        wheel_delta: delta.into(),
    }
}

/// Convenience function to return an arrays of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}

/// This function creates a temporary directory and returns a PathBuf to it.
///
/// This directory will be created relative to the executable and will therefor
/// be created in the target directory for tests when running with cargo. The
/// directory will be cleaned up at the end of the PathBufs lifetime. This
/// uses the `tempfile` crate.
#[allow(dead_code)]
#[cfg(test)]
pub fn temp_dir_for_test() -> std::path::PathBuf {
    let current_exe_path = std::env::current_exe().unwrap();
    let mut exe_dir = current_exe_path.parent().unwrap();
    if exe_dir.ends_with("deps") {
        exe_dir = exe_dir.parent().unwrap();
    }
    let test_dir = exe_dir.parent().unwrap().join("tests");
    std::fs::create_dir_all(&test_dir).unwrap();
    tempfile::Builder::new()
        .prefix("TempDir")
        .tempdir_in(test_dir)
        .unwrap()
        .into_path()
}
