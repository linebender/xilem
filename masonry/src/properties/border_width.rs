// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, UpdateCtx};
use crate::kurbo::{Point, RoundedRect, Size, Vec2};
use crate::properties::CornerRadius;

/// The width of a widget's border.
#[derive(Clone, Copy, Debug)]
pub struct BorderWidth {
    pub width: f64,
}

impl BorderWidth {
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    pub(crate) fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.width * 2., self.width * 2.))
    }

    pub(crate) fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(size.width + self.width * 2., size.height + self.width * 2.);
        let baseline = baseline + self.width;
        (size, baseline)
    }

    pub(crate) fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.width, self.width)
    }

    pub(crate) fn bg_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width)
            .to_rounded_rect(border_radius.radius - self.width)
    }

    pub(crate) fn border_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width / 2.0)
            .to_rounded_rect(border_radius.radius)
    }
}
