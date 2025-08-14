// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// Helper macro, checks that at least one item of the given `IntoIterator` matches the given predicate.
#[macro_export]
macro_rules! assert_any {
    ($iter:expr, $pred:expr $(,)?) => {
        $crate::assert_any_inner($iter, $pred)
    };
}

/// Helper macro, checks that all items of the given `IntoIterator` match the given predicate.
#[macro_export]
macro_rules! assert_all {
    ($iter:expr, $pred:expr $(,)?) => {
        $crate::assert_all_inner($iter, $pred)
    };
}

/// Helper macro, checks that no item of the given `IntoIterator` matches the given predicate.
#[macro_export]
macro_rules! assert_none {
    ($iter:expr, $pred:expr $(,)?) => {
        $crate::assert_none_inner($iter, $pred)
    };
}

use std::fmt::{Debug, Write};

#[track_caller]
#[doc(hidden)]
pub fn assert_any_inner<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
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

#[track_caller]
#[doc(hidden)]
pub fn assert_all_inner<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
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

#[track_caller]
#[doc(hidden)]
pub fn assert_none_inner<I: IntoIterator>(iter: I, mut pred: impl FnMut(I::Item) -> bool)
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
