// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// https://linebender.org/blog/doc-include
//! <!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
//! [LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_core/LICENSE
//!
//! [`alloc`]: alloc
//! [`View`]: crate::View
//! [`memoize`]: memoize
//!
//! <style>
//! .rustdoc-hidden { display: none; }
//! </style>
#![doc = include_str!("../README.md")]
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

// Used only for ad-hoc debugging of tests
#[cfg(test)]
extern crate std;

mod deferred;
pub use deferred::{AsyncCtx, MessageProxy, PhantomView, ProxyError, RawProxy};

mod view;
pub use view::{View, ViewId, ViewMarker, ViewPathTracker};

mod views;
pub use views::{
    Fork, Frozen, Lens, MapMessage, MapState, Memoize, OrphanView, RunOnce, fork, frozen, lens,
    map_action, map_message, map_state, memoize, one_of, run_once, run_once_raw,
};

mod message;
pub use message::{AnyMessage, DynMessage, MessageResult};

mod element;
pub use element::{AnyElement, Mut, NoElement, SuperElement, ViewElement};

mod any_view;
pub use any_view::{AnyView, AnyViewState};

mod sequence;
pub use sequence::{AppendVec, ElementSplice, ViewSequence};

pub mod docs;
