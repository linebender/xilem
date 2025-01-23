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
// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
// TODO: Remove any items listed as "Deferred"
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]
#![expect(clippy::allow_attributes, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
extern crate alloc;

mod deferred;
pub use deferred::AsyncCtx;
pub use deferred::MessageProxy;
pub use deferred::PhantomView;
pub use deferred::ProxyError;
pub use deferred::RawProxy;

mod view;
pub use view::View;
pub use view::ViewId;
pub use view::ViewMarker;
pub use view::ViewPathTracker;

mod views;
pub use views::adapt;
pub use views::fork;
pub use views::frozen;
pub use views::lens;
pub use views::map_action;
pub use views::map_state;
pub use views::memoize;
pub use views::one_of;
pub use views::run_once;
pub use views::run_once_raw;
pub use views::Adapt;
pub use views::AdaptThunk;
pub use views::Fork;
pub use views::Frozen;
pub use views::MapAction;
pub use views::MapState;
pub use views::Memoize;
pub use views::OrphanView;
pub use views::RunOnce;

mod message;
pub use message::DynMessage;
pub use message::Message;
pub use message::MessageResult;

mod element;
pub use element::AnyElement;
pub use element::Mut;
pub use element::NoElement;
pub use element::SuperElement;
pub use element::ViewElement;

mod any_view;
pub use any_view::AnyView;

mod sequence;
pub use sequence::AppendVec;
pub use sequence::ElementSplice;
pub use sequence::ViewSequence;

pub mod docs;
