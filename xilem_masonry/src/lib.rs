// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The Xilem views for the Masonry backend.

#![cfg_attr(not(debug_assertions), allow(unused))]
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
// https://github.com/rust-lang/rust/pull/130025
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

mod any_view;
mod one_of;
mod pod;
mod property_tuple;
mod view_ctx;
mod widget_view;

pub mod style;
pub mod view;

pub use any_view::AnyWidgetView;
pub use pod::Pod;
pub use property_tuple::PropertyTuple;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};

// FIXME - Remove these re-exports.

pub(crate) use masonry::kurbo::{Affine, Vec2};
pub(crate) use masonry::parley::Alignment as TextAlignment;
pub(crate) use masonry::peniko::Color;
pub(crate) use masonry::widgets::InsertNewline;
#[cfg(doc)]
pub(crate) use tokio;
pub(crate) use xilem_core as core;
pub(crate) use xilem_core::{MessageResult, View, ViewId};
