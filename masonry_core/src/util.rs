// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous utility functions.

use std::any::Any;

use vello::Scene;
use vello::kurbo::{Affine, Join, Shape, Stroke};
use vello::peniko::{BrushRef, Color, Fill};

/// Panic in debug and `tracing::error` in release mode.
///
/// This macro is in some way a combination of `panic` and `debug_assert`,
/// but it will log the provided message instead of ignoring it in release builds.
///
/// It's useful when a backtrace would aid debugging but a crash can be avoided in release.
#[macro_export]
macro_rules! debug_panic {
    ($msg:expr$(,)?) => {
        if cfg!(debug_assertions) {
            panic!($msg);
        } else {
            tracing::error!($msg);
        }
    };
    ($fmt:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            tracing::error!($fmt, $($arg)*);
        }
    };
}

pub use crate::debug_panic;

// ---

pub(crate) type AnyMap = anymap3::Map<dyn Any + Send + Sync>;

// --- MARK: PAINT HELPERS

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
/// Helper function for [`Scene::stroke`].
pub fn stroke<'b>(
    scene: &mut Scene,
    path: &impl Shape,
    brush: impl Into<BrushRef<'b>>,
    stroke_width: f64,
) {
    // TODO - Remove that check once we depend on a Vello release that fixes
    // https://github.com/linebender/vello/issues/662
    if stroke_width == 0.0 {
        return;
    }

    // Using Join::Miter avoids rounding corners when a widget has a wide border.
    let style = Stroke {
        width: stroke_width,
        join: Join::Miter,
        ..Default::default()
    };
    scene.stroke(&style, Affine::IDENTITY, brush, None, path);
}

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
/// Helper function for [`Scene::fill`].
pub fn fill<'b>(scene: &mut Scene, path: &impl Shape, brush: impl Into<BrushRef<'b>>) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, path);
}

/// Helper function for [`Scene::fill`] with a uniform color as the brush.
pub fn fill_color(scene: &mut Scene, path: &impl Shape, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
}

// ---

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant};

// ---

static DEBUG_COLOR: &[Color] = &[
    Color::from_rgb8(230, 25, 75),
    Color::from_rgb8(60, 180, 75),
    Color::from_rgb8(255, 225, 25),
    Color::from_rgb8(0, 130, 200),
    Color::from_rgb8(245, 130, 48),
    Color::from_rgb8(70, 240, 240),
    Color::from_rgb8(240, 50, 230),
    Color::from_rgb8(250, 190, 190),
    Color::from_rgb8(0, 128, 128),
    Color::from_rgb8(230, 190, 255),
    Color::from_rgb8(170, 110, 40),
    Color::from_rgb8(255, 250, 200),
    Color::from_rgb8(128, 0, 0),
    Color::from_rgb8(170, 255, 195),
    Color::from_rgb8(0, 0, 128),
    Color::from_rgb8(128, 128, 128),
    Color::from_rgb8(255, 255, 255),
    Color::from_rgb8(0, 0, 0),
];

/// A color used for debug painting.
///
/// The same color is always returned given the same id, usually the id of a widget.
/// When painting a widget, [`PaintCtx::debug_color`][crate::core::PaintCtx::debug_color] is typically used instead.
pub fn get_debug_color(id: u64) -> Color {
    let color_num = id as usize % DEBUG_COLOR.len();
    DEBUG_COLOR[color_num]
}

// ---

// FIXME - We're essentially completely disabling screenshots, period.
// Hopefully we'll be able to re-enable them soon.
// See https://github.com/linebender/xilem/issues/851

pub use crate::include_screenshot;

#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        // On docsrs we just remove the screenshot links for now.
        " "
    };
}

// TODO - Re-enable this once we find a way to load screenshots that doesn't go against our
// storage quotas.
#[cfg(false)]
#[cfg(docsrs)]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        concat!(
            "![", $($caption,)? "]",
            "(", "https://media.githubusercontent.com/media/linebender/xilem/",
            "masonry-v", env!("CARGO_PKG_VERSION"), "/masonry/screenshots/", $path,
            ")",
        )
    };
}

#[cfg(false)]
#[cfg(not(docsrs))]
#[doc(hidden)]
#[macro_export]
/// Macro used to create markdown img tag, with a different URL when uploading to docs.rs.
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        // This space at the start avoids triggering https://rust-lang.github.io/rust-clippy/master/index.html#suspicious_doc_comments
        // when using this macro in a `doc` attribute
        concat!(
            " ![", $($caption,)? "]",
            "(", env!("CARGO_MANIFEST_DIR"), "/screenshots/", $path, ")",
        )
    };
}
