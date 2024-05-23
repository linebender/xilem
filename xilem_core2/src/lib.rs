// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![no_std]
// TODO: Point at documentation for this pattern of README include
#![doc = concat!(
" 
<!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
[LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_core/LICENSE

<!-- intra-doc-links go here -->
<!-- TODO: If the alloc feature is disabled, this link doesn't resolve -->
[`alloc`]: alloc
[`View`]: crate::View

<style>
.rustdoc-hidden { display: none; }
</style>

<!-- Hide the header section of the README when using rustdoc -->
<div style=\"display:none\">
",
    include_str!("../README.md"),
)]

extern crate alloc;

mod element;
pub use element::{Element, SuperElement};

mod id;
pub use id::ViewId;

mod any_view;
pub use any_view::AnyView;

mod message;
pub use message::{DynMessage, Message, MessageResult};

mod model;

mod view;
pub use view::{View, ViewPathTracker};
