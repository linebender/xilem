// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Helper tools for writing unit tests.

#![cfg(not(tarpaulin_include))]

#[cfg(not(tarpaulin_include))]
mod harness;
#[cfg(not(tarpaulin_include))]
mod helper_widgets;
#[cfg(not(tarpaulin_include))]
mod screenshots;
#[cfg(not(tarpaulin_include))]
mod snapshot_utils;

pub use harness::{TestHarness, HARNESS_DEFAULT_SIZE};
pub use helper_widgets::{ModularWidget, Record, Recorder, Recording, ReplaceChild, TestWidgetExt};

use crate::WidgetId;

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
