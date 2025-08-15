// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Write};

/// Assert that `pred` returns true for at least one item in `iter`.
///
/// This provides a panic message showing the values, to aid debugging.
#[track_caller]
pub fn assert_any<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
where
    I::Item: Debug,
{
    let mut list_contents = String::new();

    for item in iter.into_iter() {
        writeln!(&mut list_contents, "  {item:?},").unwrap();

        if pred(item) {
            return;
        }
    }

    panic!("assertion failed: no item matched predicate in [\n{list_contents}]");
}

/// Assert that `pred` returns true for every item in `iter`.
///
/// This provides an error message showing which values failed the condition, to aid debugging.
#[track_caller]
pub fn assert_all<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
where
    I::Item: Debug,
{
    let mut list_contents = String::new();
    let mut error_count = 0;

    for (i, item) in iter.into_iter().enumerate() {
        let item_dbg = format!("{item:?}");

        if !pred(item) {
            writeln!(&mut list_contents, "  ({i}) {item_dbg:?},").unwrap();
            error_count += 1;
        }
        if error_count > 5 {
            writeln!(&mut list_contents, "  ...").unwrap();
            break;
        }
    }

    if error_count != 0 {
        panic!("assertion failed: items failed to match predicate: [\n{list_contents}]");
    }
}

/// Assert that `pred` returns false for every item in `iter`.
///
/// This provides an error message showing which values succeeded the condition, to aid debugging.
#[track_caller]
pub fn assert_none<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
where
    I::Item: Debug,
{
    let mut list_contents = String::new();
    let mut error_count = 0;

    for (i, item) in iter.into_iter().enumerate() {
        let item_dbg = format!("{item:?}");

        if pred(item) {
            writeln!(&mut list_contents, "  ({i}) {item_dbg:?},").unwrap();
            error_count += 1;
        }
        if error_count > 5 {
            writeln!(&mut list_contents, "  ...").unwrap();
            break;
        }
    }

    if error_count != 0 {
        panic!(
            "assertion failed: items matched predicate against expectations: [\n{list_contents}]"
        );
    }
}
