// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! An experimental library for making reactive SVG graphics.

mod app;
mod class;
mod clicked;
mod common_attrs;
mod context;
mod group;
mod kurbo_shape;
mod pointer;
mod view;
mod view_ext;

pub use peniko;
pub use peniko::kurbo;

pub use app::App;
pub use context::Cx;
pub use group::group;
pub use pointer::{PointerDetails, PointerMsg};
pub use view::{AnyView, Memoize, View, ViewMarker, ViewSequence};
pub use view_ext::ViewExt;

pub use context::ChangeFlags;

xilem_core::message!(Send);
