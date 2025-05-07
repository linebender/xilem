// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use vello::Scene;
use vello::kurbo::{Affine, Insets, Point, RoundedRect};
use vello::peniko::color::{AlphaColor, Srgb};

use crate::core::{Property, UpdateCtx};

// TODO - This is a first implementation of box shadows. A full version would need
// to address the following points:
// - Paint order: CSS shadows are drawn over neighboring boxes, which means if we want
// to emulate them, we need to paint them after sibling widgets. This would require
// adding some kind of post_paint pass.
// - Inset shadows: CSS shadows can be either drop shadows (behind element) or inset
// shadows (inside element). We should implement both and add an `inset` attribute.
// - Spread radius: CSS shadow can change size without changing the blur level using
// a "spread radius" value. We should implement it and add a `spread_radius` value.
// - Corner radius: Right now take our widget's corner radii, and average them to draw a shadow with a single corner radius. Ideally we'd like to match individual values.

/// The drop shadow of a Widget.
///
/// Will be invisible if default values are kept.
#[derive(Clone, Copy, Debug)]
pub struct BoxShadow {
    /// The shadow's color.
    pub color: AlphaColor<Srgb>,

    /// The offset from the widget to the shadow. A value of zero means the shadow will be exactly be aligned with its widget.
    pub offset: Point,

    /// The distance between the shadow's "inner edge" and the closest fully-transparent point.
    ///
    /// A value of zero means the shadow's edge will be shard.
    /// Negative values will be treated as zero.
    pub blur_radius: f64,
}

impl Property for BoxShadow {
    fn static_default() -> &'static Self {
        static DEFAULT: BoxShadow = BoxShadow {
            color: AlphaColor::TRANSPARENT,
            offset: Point::ZERO,
            blur_radius: 0.,
        };
        &DEFAULT
    }
}

impl Default for BoxShadow {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl BoxShadow {
    /// Create a new shadow with the given color and offset.
    pub fn new(color: AlphaColor<Srgb>, offset: impl Into<Point>) -> Self {
        Self {
            color,
            offset: offset.into(),
            blur_radius: 0.,
        }
    }

    /// Builder method to change the shadow's blur radius.
    pub fn blur(self, blur_radius: f64) -> Self {
        Self {
            blur_radius,
            ..self
        }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        // TODO - request_paint_only?
        ctx.request_layout();
    }

    /// Returns false if the shadow can be safely treated as non-existent.
    ///
    /// May have false positives.
    pub fn is_visible(&self) -> bool {
        let alpha = self.color.components[3];
        alpha != 0.0
    }

    /// Helper function to paint the shadow into a scene.
    pub fn paint(&self, scene: &mut Scene, transform: Affine, rect: RoundedRect) {
        if !self.is_visible() {
            return;
        }

        let transform = transform.pre_translate(self.offset.to_vec2());
        let blur_radius = self.blur_radius.max(0.);

        let radius = (rect.radii().bottom_left
            + rect.radii().bottom_right
            + rect.radii().top_left
            + rect.radii().top_right)
            / 4.;
        scene.draw_blurred_rounded_rect(
            transform,
            rect.rect(),
            self.color,
            radius,
            // TODO - I'm not sure this is the right std_dev.
            blur_radius,
        );
    }

    /// Helper function that returns how much a given shadow expands the paint rect.
    pub fn get_insets(&self) -> Insets {
        let blur_radius = self.blur_radius.max(0.);
        Insets {
            x0: (blur_radius - self.offset.x).max(0.),
            y0: (blur_radius - self.offset.y).max(0.),
            x1: (blur_radius + self.offset.x).max(0.),
            y1: (blur_radius + self.offset.y).max(0.),
        }
    }
}
