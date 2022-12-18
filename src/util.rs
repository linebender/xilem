// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Miscellaneous utility functions.

#![cfg(not(tarpaulin_include))]

use std::any::Any;
use std::hash::Hash;

/// Panic in debug and tracing::error in release mode.
///
/// This macro is in some way a combination of `panic` and `debug_assert`,
/// but it will log the provided message instead of ignoring it in release builds.
///
/// It's useful when a backtrace would aid debugging but a crash can be avoided in release.
macro_rules! debug_panic {
    () => { ... };
    ($msg:expr) => {
        if cfg!(debug_assertions) {
            panic!($msg);
        } else {
            tracing::error!($msg);
        }
    };
    ($msg:expr,) => { debug_panic!($msg) };
    ($fmt:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            tracing::error!($fmt, $($arg)*);
        }
    };
}

// ---

/// An enum for specifying whether an event was handled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Handled {
    /// An event was already handled, and shouldn't be propagated to other event handlers.
    Yes,
    /// An event has not yet been handled.
    No,
}

impl Handled {
    /// Has the event been handled yet?
    pub fn is_handled(self) -> bool {
        self == Handled::Yes
    }
}

impl From<bool> for Handled {
    /// Returns `Handled::Yes` if `handled` is true, and `Handled::No` otherwise.
    fn from(handled: bool) -> Handled {
        if handled {
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

// ---

/// Trait extending Any, implemented for all types that implement Any.
///
/// This is a band-aid to substitute for a lack of dyn trait upcasting.
pub trait AsAny: Any {
    /// Return self.
    fn as_dyn_any(&self) -> &dyn Any;
    /// Return self.
    fn as_mut_dyn_any(&mut self) -> &mut dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_dyn_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_dyn_any(&mut self) -> &mut dyn Any {
        self
    }
}
