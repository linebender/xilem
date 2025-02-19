// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types and logic commonly used across widgets.

// TODO - Add link to doc

#![allow(
    missing_docs,
    reason = "A lot of properties and especially their fields are self-explanatory."
)]

use vello::peniko::color::{AlphaColor, Srgb};

use crate::core::{MutateCtx, WidgetProperty};

// TODO - Split out into files.

/// The background color of a widget.
#[derive(Clone, Copy, Debug)]
pub struct BackgroundColor {
    pub color: AlphaColor<Srgb>,
}

impl WidgetProperty for BackgroundColor {
    fn changed(ctx: &mut MutateCtx<'_>) {
        ctx.request_paint_only();
    }
}
