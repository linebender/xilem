// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, UpdateCtx};
use crate::kurbo::{Point, Size, Vec2};

/// The width of padding between a widget's border and its contents.
#[expect(missing_docs, reason = "obvious")]
#[derive(Clone, Copy, Debug)]
pub struct Padding {
    pub x: f64,
    pub y: f64,
}

impl Padding {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Shrinks the box constraints by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.x * 2., self.y * 2.))
    }

    /// Expands the size and raises the baseline by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(size.width + self.x * 2., size.height + self.y * 2.);
        let baseline = baseline + self.y;
        (size, baseline)
    }

    /// Shifts the position by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.x, self.y)
    }
}
