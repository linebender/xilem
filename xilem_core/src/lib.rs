// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Xilem Core provides primitives which are used by [Xilem][] (a cross-platform GUI toolkit) and [Xilem Web][] (a web frontend framework).
//! If you are using Xilem, [its documentation][xilem docs] will probably be more helpful for you. <!-- TODO: In the long-term, we probably also need a book? -->
//!
//! Xilem apps will interact with some of the functions from this crate, in particular [`memoize`][].
//! Xilem apps which use custom widgets (and therefore must implement custom views), will implement the [`View`][] trait.
//!
//! If you wish to implement the Xilem pattern in a different domain (such as for a terminal user interface), this crate can be used to do so.
//! Though, while Xilem Core should be able to support all kinds of domains, the crate prioritizes the ergonomics for users of Xilem.
//!
//! ## Hot reloading
//!
//! Xilem Core does not currently include infrastructure to enable hot reloading, but this is planned.
//! The current proposal would split the application into two processes:
//!
//!  - The app process, which contains the app state and create the views, which would be extremely lightweight and can be recompiled and restarted quickly.
//!  - The display process, which contains the widgets and would be long-lived, updating to match the new state of the view tree provided by the app process.
//!
//! ## Quickstart
//!
//! <!-- TODO? -->
//!
//! ## `no_std` support
//!
//! Xilem Core supports running with `#![no_std]`, but does require [`alloc`][] to be available.
//!
//! [Xilem]: https://crates.io/crates/xilem
//! [Xilem Web]: https://crates.io/crates/xilem_web
//! [xilem docs]: https://docs.rs/xilem/latest/xilem/
//! [Zulip]: https://xi.zulipchat.com/#narrow/stream/354396-xilem

// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![no_std]
// TODO: Remove any items listed as "Deferred"
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
extern crate alloc;

// // Used only for ad-hoc debugging of tests
// #[cfg(test)]
// extern crate std;

pub use anymore;

mod context;
pub use context::MessageContext;

mod deferred;
pub use deferred::{AsyncCtx, MessageProxy, PhantomView, ProxyError, RawProxy};

mod environment;
pub use environment::{
    Environment, Provides, Rebuild, Resource, WithContext, provides, with_context,
};

mod view;
pub use view::{View, ViewId, ViewMarker, ViewPathTracker};

mod views;
pub use views::{
    Fork, Frozen, Lens, MapMessage, MapState, Memoize, OrphanView, RunOnce, fork, frozen, lens,
    map_action, map_message, map_state, memoize, one_of, run_once, run_once_raw,
};

mod message;
pub use message::{DynMessage, MessageResult, SendMessage};

mod element;
pub use element::{AnyElement, Mut, NoElement, SuperElement, ViewElement};

mod any_view;
pub use any_view::{AnyView, AnyViewState};

mod sequence;
pub use sequence::{AppendVec, ElementSplice, ViewSequence, WithoutElements, without_elements};

pub mod docs;
