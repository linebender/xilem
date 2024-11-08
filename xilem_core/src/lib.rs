// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(test), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
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
// LINEBENDER LINT SET - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// TODO: Remove any items listed as "Deferred"
#![expect(
    rustdoc::broken_intra_doc_links,
    reason = "Deferred: Noisy. Tracked in https://github.com/linebender/xilem/issues/449"
)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(unused_qualifications, reason = "Deferred: Noisy")]
#![expect(single_use_lifetimes, reason = "Deferred: Noisy")]
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(clippy::use_self, reason = "Deferred: Noisy")]
#![expect(clippy::missing_errors_doc, reason = "Can be quite noisy?")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]
#![expect(clippy::allow_attributes, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
extern crate alloc;

mod deferred;
pub use deferred::{AsyncCtx, MessageProxy, PhantomView, ProxyError, RawProxy};

mod view;
pub use view::{View, ViewId, ViewMarker, ViewPathTracker};

mod views;
pub use views::{
    adapt, fork, frozen, lens, map_action, map_state, memoize, one_of, run_once, run_once_raw,
    Adapt, AdaptThunk, Fork, Frozen, MapAction, MapState, Memoize, OrphanView, RunOnce,
};

mod message;
pub use message::{DynMessage, Message, MessageResult};

mod element;
pub use element::{AnyElement, Mut, NoElement, SuperElement, ViewElement};

mod any_view;
pub use any_view::AnyView;

mod sequence;
pub use sequence::{AppendVec, ElementSplice, ViewSequence};

pub mod docs;
