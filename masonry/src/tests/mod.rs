// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! We test core Masonry features and passes here instead of in `masonry_testing`,
//! both to centralize tests in a single crate and to have access to the `masonry`
//! widget/property set in our tests if needed.

use crate::kurbo::Rect;

mod accessibility;
mod action;
mod anim;
mod compose;
mod event;
mod layout;
mod mutate;
mod paint;
mod properties;
mod update;
mod widget_tag;

#[track_caller]
pub(crate) fn assert_approx_eq(name: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "{name}: expected {expected}, got {actual}"
    );
}

#[track_caller]
pub(crate) fn assert_rect_approx_eq(name: &str, actual: Rect, expected: Rect) {
    assert_approx_eq(&format!("{name}.x0"), actual.x0, expected.x0);
    assert_approx_eq(&format!("{name}.y0"), actual.y0, expected.y0);
    assert_approx_eq(&format!("{name}.x1"), actual.x1, expected.x1);
    assert_approx_eq(&format!("{name}.y1"), actual.y1, expected.y1);
}
