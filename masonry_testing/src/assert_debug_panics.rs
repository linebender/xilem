// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// Checks that the given expression panics in debug mode. No-op in release mode.
///
/// This macro is useful for tests that check the behavior of `debug_assert!` or `debug_panic!` calls.
#[macro_export]
macro_rules! assert_debug_panics {
    ($expr:expr) => {
        $crate::assert_debug_panics_inner(
            || {
                $expr;
            },
            "".into(),
        )
    };

    ($expr:expr, $needle:expr) => {
        $crate::assert_debug_panics_inner(
            || {
                $expr;
            },
            ($needle).to_string(),
        )
    };
}

use std::panic::{AssertUnwindSafe, catch_unwind};

#[track_caller]
#[doc(hidden)]
pub fn assert_debug_panics_inner(callback: impl FnOnce(), needle: String) {
    if cfg!(not(debug_assertions)) {
        return;
    }

    // This function is only meant to be used in tests, so the potential for breakage is limited.
    // Note that AssertUnwindSafe is not a safety invariant: if someone misuses this somehow,
    // they may bump into functional bugs, but never undefined behavior.
    // See https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html for details.
    let callback = AssertUnwindSafe(callback);

    let res = catch_unwind(callback);

    let Err(err) = res else {
        panic!("test did not panic as expected");
    };

    // The panic payload is virtually always a `&'static str` or `String`.
    // See https://doc.rust-lang.org/src/std/panic.rs.html for details
    // Or https://github.com/rust-lang/rust/blob/213d946a384b46989f6fd9c8ae9c547b4e354455/library/std/src/panic.rs#L62-L69

    let message;
    if let Some(s) = err.downcast_ref::<&str>() {
        message = s.to_string();
    } else if let Some(s) = err.downcast_ref::<String>() {
        message = s.clone();
    } else {
        panic!("panic had unexpected type");
    }

    if !message.contains(&needle) {
        panic!(
            concat!(
                "panic did not contain expected string\n",
                "      panic message: {}\n",
                " expected substring: {}",
            ),
            message, needle,
        );
    }
}
