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

pub trait AsAny: Any {
    fn as_dyn_any(&self) -> &dyn Any;
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
