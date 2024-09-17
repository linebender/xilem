// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub, unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]
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
