// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::Scene;

use crate::core::{PaintCtx, PropertiesRef};
use crate::kurbo::{Affine, Size};
use crate::peniko::Fill;
use crate::properties::{
    ActiveBackground, Background, BorderWidth, BoxShadow, CornerRadius, DisabledBackground,
};

/// References to common pre-paint properties.
pub struct PrePaintProps<'a> {
    /// Box shadow.
    pub box_shadow: &'a BoxShadow,
    /// Background.
    ///
    /// Considers disabled and active state.
    pub background: &'a Background,
    /// Border width.
    pub border_width: &'a BorderWidth,
    /// Corner radius,
    pub corner_radius: &'a CornerRadius,
}

impl<'a> PrePaintProps<'a> {
    /// Returns common pre-paint properties based on widget state.
    pub fn fetch(ctx: &mut PaintCtx<'_>, props: &'a PropertiesRef<'_>) -> Self {
        let box_shadow = props.get::<BoxShadow>();
        let background = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else if ctx.is_active() {
            &props.get::<ActiveBackground>().0
        } else {
            props.get::<Background>()
        };
        let border_width = props.get::<BorderWidth>();
        let corner_radius = props.get::<CornerRadius>();

        Self {
            box_shadow,
            background,
            border_width,
            corner_radius,
        }
    }
}

/// Paints the widget's box shadow and background.
pub fn pre_paint(ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
    let size = ctx.size();
    let p = PrePaintProps::fetch(ctx, props);

    paint_box_shadow(scene, size, p.box_shadow, p.corner_radius);
    paint_background(scene, size, p.background, p.border_width, p.corner_radius);
}

/// Paints the widget's box shadow.
pub fn paint_box_shadow(
    scene: &mut Scene,
    size: Size,
    box_shadow: &BoxShadow,
    corner_radius: &CornerRadius,
) {
    if box_shadow.is_visible() {
        let box_shadow_rect = box_shadow.shadow_rect(size, corner_radius);
        box_shadow.paint(scene, Affine::IDENTITY, box_shadow_rect);
    }
}

/// Paints the widget's background.
pub fn paint_background(
    scene: &mut Scene,
    size: Size,
    background: &Background,
    border_width: &BorderWidth,
    corner_radius: &CornerRadius,
) {
    // TODO: Fix remaining issues, see https://github.com/linebender/xilem/issues/1592
    //    1. Figure out how to skip painting fully transparent backgrounds.
    //    2. Don't subtract the border from the background rect. Will need solution for border
    //       painting, as background should go exactly to the outer border and not beyond.
    let bg_rect = border_width.bg_rect(size, corner_radius);
    let bg_brush = background.get_peniko_brush_for_rect(bg_rect.rect());
    scene.fill(Fill::NonZero, Affine::IDENTITY, &bg_brush, None, &bg_rect);
}
