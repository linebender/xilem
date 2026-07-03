// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::layout::Length;
use crate::peniko::Color;

/// The thickness of a track.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct TrackThickness(pub Length);

impl Property for TrackThickness {
    fn static_default() -> &'static Self {
        static DEFAULT: TrackThickness = TrackThickness(Length::ZERO);
        &DEFAULT
    }
}

impl TrackThickness {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_layout();
        }
    }
}

/// Colors of a track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackColor {
    /// Color of the active portion.
    pub active: Color,

    /// Color of the inactive portion.
    pub inactive: Color,
}

impl Property for TrackColor {
    fn static_default() -> &'static Self {
        static DEFAULT: TrackColor = TrackColor {
            active: Color::WHITE,
            inactive: Color::WHITE.with_alpha(0.5),
        };
        &DEFAULT
    }
}

impl Default for TrackColor {
    fn default() -> Self {
        *Self::static_default()
    }
}
