// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
// TODO: Point at documentation for this pattern of README include.
// It has some neat advantages but is quite esoteric
#![doc = concat!(
" 
<!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
[LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_core/LICENSE

<!-- intra-doc-links go here -->
<!-- TODO: If the alloc feature is disabled, this link doesn't resolve -->
[`alloc`]: alloc
[`View`]: crate::View
[`memoize`]: memoize

<style>
.rustdoc-hidden { display: none; }
</style>

<!-- Hide the header section of the README when using rustdoc -->
<div style=\"display:none\">
",
    include_str!("../README.md"),
)]

extern crate alloc;

mod view;
pub use view::{View, ViewId, ViewPathTracker};

mod views;
pub use views::{
    memoize, Adapt, AdaptThunk, AsOrphanView, Memoize, OneOf2, OneOf2Ctx, OneOf3, OneOf3Ctx,
    OneOf4, OneOf4Ctx, OneOf5, OneOf5Ctx, OneOf6, OneOf6Ctx, OneOf7, OneOf7Ctx, OneOf8, OneOf8Ctx,
    OneOf9, OneOf9Ctx, OrphanView,
};

mod message;
pub use message::{DynMessage, Message, MessageResult};

mod element;
pub use element::{AnyElement, Mut, SuperElement, ViewElement};

mod any_view;
pub use any_view::AnyView;

mod sequence;
pub use sequence::{AppendVec, ElementSplice, ViewSequence};
