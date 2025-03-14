// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, UpdateCtx};
use crate::kurbo::{Point, Size, Vec2};

/// The width of padding between a widget's border and its contents.
#[derive(Clone, Copy, Debug)]
pub struct Padding {
    pub x: f64,
    pub y: f64,
}

impl Padding {
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    pub(crate) fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.x * 2., self.y * 2.))
    }

    pub(crate) fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(size.width + self.x * 2., size.height + self.y * 2.);
        let baseline = baseline + self.y;
        (size, baseline)
    }

    pub(crate) fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.x, self.y)
    }
}
