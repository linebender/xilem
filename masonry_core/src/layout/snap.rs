// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Affine, Point, Rect, Vec2};

/// Cache key for axis-aligned layout-time snapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SnapKey {
    /// X scale from border-box space to window space.
    pub(crate) scale_x: f64,
    /// Y scale from border-box space to window space.
    pub(crate) scale_y: f64,
    /// Window space pixel x translation's fractional part.
    pub(crate) translation_x: f64,
    /// Window space pixel y translation's fractional part.
    pub(crate) translation_y: f64,
}

impl SnapKey {
    /// Creates a new snap key based on the provided `snap_transform`.
    ///
    /// The provided `snap_transform` must support snapping,
    /// which can be checked via [`supports_box_snapping`].
    ///
    /// # Panics
    ///
    /// Panics if `snap_transform` does not support snapping and debug assertions are enabled.
    pub(crate) fn new(snap_transform: Affine, scale_factor: f64) -> Self {
        let local_to_device = snap_transform.then_scale(scale_factor);
        debug_assert!(
            supports_box_snapping(local_to_device),
            "snap key creation attempted with an incompatible transform"
        );

        let [scale_x, _, _, scale_y, translation_x, translation_y] = local_to_device.as_coeffs();

        Self {
            scale_x,
            scale_y,
            translation_x: translation_x.rem_euclid(1.0),
            translation_y: translation_y.rem_euclid(1.0),
        }
    }
}

/// Returns whether the transform supports box snapping.
///
/// Box snapping is supported when the transform maps local widget axes to device
/// axes without rotation or shear. Scaling, translation, and axis flips are fine.
pub(crate) fn supports_box_snapping(transform: Affine) -> bool {
    let [a, b, c, d, _, _] = transform.as_coeffs();

    // Kurbo affine coefficients represent:
    //
    // x' = a*x + c*y + e
    // y' = b*x + d*y + f
    //
    // The off-diagonal coefficients b and c must be zero. If either is non-zero, x contributes
    // to output y or y contributes to output x. That means the transform mixes axes, as in
    // rotation, shear, or axis swapping, which this snapping path intentionally does not support.
    //
    // The scale coefficients a and d must be non-zero so the transform can be inverted
    // when mapping snapped device edges back to local coordinates.
    //
    // The translation coefficients e and f do not affect whether edges stay axis-aligned,
    // so they are intentionally ignored.
    b == 0. && c == 0. && a != 0. && d != 0.
}

/// Snaps the given `border_box` to device pixel edges.
///
/// The provided `snap_transform` must support snapping,
/// which can be checked via [`supports_box_snapping`].
///
/// # Panics
///
/// Panics if `snap_transform` does not support snapping and debug assertions are enabled.
pub(crate) fn snap_border_box(border_box: Rect, snap_transform: Affine, scale_factor: f64) -> Rect {
    let local_to_device = snap_transform.then_scale(scale_factor);
    debug_assert!(
        supports_box_snapping(local_to_device),
        "box snapping attempted with an incompatible transform"
    );

    let device_border_box = local_to_device.transform_rect_bbox(border_box);
    let snapped_device_border_box = Rect::new(
        device_border_box.x0.round(),
        device_border_box.y0.round(),
        device_border_box.x1.round(),
        device_border_box.y1.round(),
    );

    let device_to_local = local_to_device.inverse();
    Rect::from_points(
        device_to_local * Point::new(snapped_device_border_box.x0, snapped_device_border_box.y0),
        device_to_local * Point::new(snapped_device_border_box.x1, snapped_device_border_box.y1),
    )
}

/// Snaps a local translation delta to an integer device-pixel delta.
///
/// The provided `snap_transform` must support snapping,
/// which can be checked via [`supports_box_snapping`].
///
/// # Panics
///
/// Panics if `snap_transform` does not support snapping and debug assertions are enabled.
pub(crate) fn snap_translation_delta(
    delta: Vec2,
    snap_transform: Affine,
    scale_factor: f64,
) -> Vec2 {
    let local_to_device = snap_transform.then_scale(scale_factor);
    debug_assert!(
        supports_box_snapping(local_to_device),
        "translation delta snapping attempted with an incompatible transform"
    );

    let device_origin = local_to_device * Point::ORIGIN;
    let device_delta_point = local_to_device * Point::new(delta.x, delta.y);
    let device_delta = device_delta_point - device_origin;
    let snapped_device_delta = Vec2::new(device_delta.x.round(), device_delta.y.round());

    let device_to_local = local_to_device.inverse();
    let local_origin = device_to_local * Point::ORIGIN;
    let local_delta_point =
        device_to_local * Point::new(snapped_device_delta.x, snapped_device_delta.y);
    local_delta_point - local_origin
}
