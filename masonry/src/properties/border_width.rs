// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, Property, UpdateCtx};
use crate::properties::CornerRadius;
use crate::util::stroke;

use vello::Scene;
use vello::kurbo::{self, Affine, Cap, Join, Line, Point, RoundedRect, Size, Stroke, Vec2};
use vello::peniko::Color;

/// Border widths for each side of a widget, in logical pixels.
/// Order follows CSS convention: Top, Right, Bottom, Left.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct BorderWidth {
    /// Thickness of the top border.
    pub top: f64,
    /// Thickness of the right border.
    pub right: f64,
    /// Thickness of the bottom border.
    pub bottom: f64,
    /// Thickness of the left border.
    pub left: f64,
}

impl Property for BorderWidth {
    fn static_default() -> &'static Self {
        static DEFAULT: BorderWidth = BorderWidth {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        };
        &DEFAULT
    }
}

impl BorderWidth {
    /// Creates a new `BorderWidth` with (Top, Right, Bottom, Left) values.
    pub const fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates a uniform border with the same thickness on all sides.
    pub fn uniform(width: f64) -> Self {
        Self {
            top: width,
            right: width,
            bottom: width,
            left: width,
        }
    }

    /// Creates a symmetric border with vertical and horizontal values.
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Returns the minimum side thickness.
    pub fn min_side(&self) -> f64 {
        self.top.min(self.right).min(self.bottom).min(self.left)
    }

    /// Returns true if all sides are equal within a small epsilon.
    pub fn is_uniform(&self) -> bool {
        let eps = 1e-9;
        (self.top - self.right).abs() < eps
            && (self.top - self.bottom).abs() < eps
            && (self.top - self.left).abs() < eps
    }

    /// Requests a layout update if this property type has changed.
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_layout();
        }
    }

    /// Shrinks the box constraints by the border width.
    pub fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.left + self.right, self.top + self.bottom))
    }

    /// Expands the size and raises the baseline by the border width.
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let new_size = Size::new(
            size.width + self.left + self.right,
            size.height + self.top + self.bottom,
        );
        (new_size, baseline + self.top)
    }

    /// Shifts the position by the border width.
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.left, self.top)
    }

    /// Rounded rectangle inset by the full border width (for background fill).
    pub fn bg_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        let insets = kurbo::Insets::new(self.left, self.top, self.right, self.bottom);
        let inner_radius = (border_radius.radius - self.min_side()).max(0.0);
        size.to_rect().inset(insets).to_rounded_rect(inner_radius)
    }

    /// Rounded rectangle inset by half the border width (for centered stroke).
    pub fn border_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        let insets = kurbo::Insets::new(
            self.left / 2.0,
            self.top / 2.0,
            self.right / 2.0,
            self.bottom / 2.0,
        );
        size.to_rect()
            .inset(insets)
            .to_rounded_rect(border_radius.radius)
    }

    /// Paints the border. Uses rounded rect stroke for uniform borders,
    /// and explicit line strokes for non-uniform borders.
    pub fn paint(&self, scene: &mut Scene, border_rect: &RoundedRect, color: Color, size: Size) {
        if self.is_uniform() {
            if self.top > 0.0 {
                stroke(scene, border_rect, color, self.top);
            }
        } else {
            let r = size.to_rect();

            // Top
            Self::stroke_side(
                scene,
                (r.x0, r.y0 + self.top / 2.0),
                (r.x1, r.y0 + self.top / 2.0),
                color,
                self.top,
            );

            // Bottom
            Self::stroke_side(
                scene,
                (r.x0, r.y1 - self.bottom / 2.0),
                (r.x1, r.y1 - self.bottom / 2.0),
                color,
                self.bottom,
            );

            // Left
            Self::stroke_side(
                scene,
                (r.x0 + self.left / 2.0, r.y0),
                (r.x0 + self.left / 2.0, r.y1),
                color,
                self.left,
            );

            // Right
            Self::stroke_side(
                scene,
                (r.x1 - self.right / 2.0, r.y0),
                (r.x1 - self.right / 2.0, r.y1),
                color,
                self.right,
            );
        }
    }

    /// Helper to stroke a single side with flat caps and sharp joins.
    fn stroke_side(
        scene: &mut Scene,
        p0: impl Into<Point>,
        p1: impl Into<Point>,
        color: Color,
        width: f64,
    ) {
        if width > 0.0 {
            let line = Line::new(p0, p1);
            let mut style = Stroke::new(width);
            style.start_cap = Cap::Butt;
            style.end_cap = Cap::Butt;
            style.join = Join::Miter;
            scene.stroke(&style, Affine::IDENTITY, color, None, &line);
        }
    }
}

impl From<f64> for BorderWidth {
    fn from(w: f64) -> Self {
        Self::uniform(w)
    }
}
impl From<(f64, f64)> for BorderWidth {
    fn from((v, h): (f64, f64)) -> Self {
        Self::symmetric(v, h)
    }
}
impl From<(f64, f64, f64, f64)> for BorderWidth {
    fn from((t, r, b, l): (f64, f64, f64, f64)) -> Self {
        Self::new(t, r, b, l)
    }
}

