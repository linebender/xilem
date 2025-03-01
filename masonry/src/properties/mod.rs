// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types and logic commonly used across widgets.
//!
//! See [properties documentation](crate::doc::doc_03_implementing_container_widget) for details.

#![allow(
    missing_docs,
    reason = "A lot of properties and especially their fields are self-explanatory."
)]

use std::any::TypeId;

use vello::peniko::color::{AlphaColor, Srgb};

use crate::core::UpdateCtx;

// TODO - Split out into files.

/// The background color of a widget.
#[derive(Clone, Copy, Debug)]
pub struct BackgroundColor {
    pub color: AlphaColor<Srgb>,
}

impl BackgroundColor {
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_paint_only();
        }
    }
}
