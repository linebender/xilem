// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{PaintCtx, PropertiesRef, PropertyCache};
use crate::imaging::Painter;
use crate::kurbo::{Affine, Join, Rect, Stroke};
use crate::properties::{Background, BorderColor, BorderWidth, BoxShadow, CornerRadius};

/// References to common pre-paint properties.
#[derive(Debug)]
pub struct PrePaintProps<'a> {
    /// Box shadow.
    pub box_shadow: &'a BoxShadow,
    /// Background.
    ///
    /// Considers disabled and active state.
    pub background: &'a Background,
    /// Border width.
    pub border_width: &'a BorderWidth,
    /// Border color.
    ///
    /// Considers focus and hovered state.
    pub border_color: &'a BorderColor,
    /// Corner radius,
    pub corner_radius: &'a CornerRadius,
}

impl<'a> PrePaintProps<'a> {
    /// Returns common pre-paint properties based on widget state.
    pub fn fetch(props: &'a PropertiesRef<'_>, cache: &mut PropertyCache) -> Self {
        let box_shadow = props.get::<BoxShadow>(cache);
        let background = props.get::<Background>(cache);
        let border_color = props.get::<BorderColor>(cache);
        let border_width = props.get::<BorderWidth>(cache);
        let corner_radius = props.get::<CornerRadius>(cache);

        Self {
            box_shadow,
            background,
            border_width,
            border_color,
            corner_radius,
        }
    }
}

/// Paints the widget's box shadow, background, and border.
pub fn pre_paint(ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
    let bbox = ctx.border_box();
    let cache = ctx.property_cache();
    let p = PrePaintProps::fetch(props, cache);

    paint_box_shadow(painter, bbox, p.box_shadow, p.corner_radius);
    paint_background(painter, bbox, p.background, p.border_width, p.corner_radius);
    paint_border(
        painter,
        bbox,
        p.border_color,
        p.border_width,
        p.corner_radius,
    );
}

/// Paints the widget's box shadow.
pub fn paint_box_shadow(
    painter: &mut Painter<'_>,
    border_box: Rect,
    box_shadow: &BoxShadow,
    corner_radius: &CornerRadius,
) {
    if !box_shadow.is_visible() {
        return;
    }
    let box_shadow_rect = border_box.to_rounded_rect(corner_radius.radius.get());
    box_shadow.paint(painter, Affine::IDENTITY, box_shadow_rect);
}

/// Paints the widget's background.
pub fn paint_background(
    painter: &mut Painter<'_>,
    border_box: Rect,
    background: &Background,
    border_width: &BorderWidth,
    corner_radius: &CornerRadius,
) {
    if !background.is_visible() {
        return;
    }
    // TODO: Fix remaining issues, see https://github.com/linebender/xilem/issues/1592
    //    1. Don't subtract the border from the background rect. Will need solution for border
    //       painting, as background should go exactly to the outer border and not beyond.
    let bg_rect = border_width.bg_rect(border_box, corner_radius);
    let bg_brush = background.get_peniko_brush_for_rect(bg_rect.rect());
    painter.fill(bg_rect, &bg_brush).draw();
}

/// Paints the widget's border.
pub fn paint_border(
    painter: &mut Painter<'_>,
    border_box: Rect,
    border_color: &BorderColor,
    border_width: &BorderWidth,
    corner_radius: &CornerRadius,
) {
    let border_width_value = border_width.width.get();
    if border_width_value == 0. || !border_color.is_visible() {
        return;
    }
    let border_rect = border_width.border_rect(border_box, corner_radius);
    // Using Join::Miter avoids rounding corners when a widget has a wide border.
    let border_style = Stroke {
        width: border_width_value,
        join: Join::Miter,
        ..Default::default()
    };
    painter
        .stroke(border_rect, &border_style, border_color.color)
        .draw();
}
