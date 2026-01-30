// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Core properties.

mod background;
mod border_color;
mod border_width;
mod box_shadow;
mod corner_radius;
mod dimensions;
mod padding;

pub mod types;

use std::any::TypeId;

pub use background::*;
pub use border_color::*;
pub use border_width::*;
pub use box_shadow::*;
pub use corner_radius::*;
pub use dimensions::*;
pub use padding::*;

use crate::core::{Property, UpdateCtx};

/// Handles core property changes.
pub(crate) fn core_property_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
    // TODO: For BoxShadow we'd like to request a mere pre-paint pass.
    //       However, it affects the size of the paint rect, which is handled in layout.
    if Dimensions::matches(property_type)
        || BoxShadow::matches(property_type)
        || BorderWidth::matches(property_type)
        || CornerRadius::matches(property_type)
        || Padding::matches(property_type)
    {
        ctx.request_layout();
    } else if DisabledBackground::matches(property_type)
        || ActiveBackground::matches(property_type)
        || Background::matches(property_type)
        || FocusedBorderColor::matches(property_type)
        || HoveredBorderColor::matches(property_type)
        || BorderColor::matches(property_type)
        || BorderWidth::matches(property_type)
        || CornerRadius::matches(property_type)
    {
        ctx.request_pre_paint();
    }
}
