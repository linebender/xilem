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
pub(crate) type TypeSet = std::collections::HashSet<
    std::any::TypeId,
    std::hash::BuildHasherDefault<anymap3::TypeIdHasher>,
>;

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

/// Convert a 2d rectangle from Parley to one used for drawing in Vello and other maths.
pub fn bounding_box_to_rect(bb: parley::BoundingBox) -> vello::kurbo::Rect {
    vello::kurbo::Rect {
        x0: bb.x0,
        y0: bb.y0,
        x1: bb.x1,
        y1: bb.y1,
    }
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

// Macros are exported from crate root by default. Re-export them from here.
pub use crate::include_screenshot;
pub use crate::include_screenshot_reference;

// If we made this into proc macros, we would gain the following features:
// 1) Automatic detection of the file existing - see https://github.com/linebender/xilem/issues/1080
// 2) Extract the "repository" from CARGO_PKG_REPOSITORY and auto-generate the online URL version.

// We want to show the "local" image if it's present (e.g. from a git dependency or in the local repository).
// The image won't be available locally if our docs are being built on docs.rs or from a crates.io dependency,
// as we don't include the screenshots in the published package (for space/bandwidth reasons).
// This fall back uses `raw.githubusercontent.com`, which allows it to access the correct version of the screenshot for the crate's version.
// Unfortunately, it isn't currently possible to detect that this fallback is needed (without a procedural macro or build script);
// as such, we currently use `cfg(docsrs)` as a proxy for whether to use a fallback.
// This does mean that screenshots may fail to display in some cases, e.g. the user is pulling a crate as a
// crates.io dependency and then generating its doc locally.
// Masonry's documentation has a few warnings for these cases.

/// Markdown content to display a screenshot from the current crate's `screenshots` directory.
///
/// This can be added to docs as follows:
///
/// ```rust,ignore
/// /// Some docs here.
/// ///
/// #[doc = include_screenshot!("button_hello.png", "Button with text label.")]
/// ```
///
/// The caption should have a full-stop at the end, as it's being used as alt-text.
///
/// **Warning: This macro will only function correctly for packages in the Xilem repository,
/// as it hardcodes the supported GitHub repository.**
#[cfg(not(docsrs))]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        concat!(
            "![", $($caption,)? "]",
            "(", env!("CARGO_MANIFEST_DIR"), "/screenshots/", $path, ")",
        )
    };
}

#[cfg(docsrs)]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        concat!(
            "![", $($caption,)? "]",
            // The online path to the screenshot, on this released version.
            // Ideally, the "base URL" would be customisable, so end-users could use this macro too.x
            // The `v` is because of our tag name convention.
            "(https://raw.githubusercontent.com/linebender/xilem/v", env!("CARGO_PKG_VERSION"), "/", env!("CARGO_PKG_NAME"), "/screenshots/", $path, ")",
        )
    };
}

/// Markdown content to provide a screenshot from the current crate's `screenshots` directory as a [Markdown link reference definition](https://spec.commonmark.org/0.31.2/#link-reference-definition).
///
/// This can be added to docs as follows:
///
/// ```rust,ignore
/// /// Some docs here.
/// ///
/// /// ![Alt text][my-screenshot]
/// ///
/// #[doc = include_screenshot_reference!("my-screenshot", "button_hello.png"]
/// ```
///
/// **Warning: This macro will only function correctly for packages in the Xilem repository,
/// as it hardcodes the supported GitHub repository.**
#[cfg(not(docsrs))]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot_reference {
    ($label:literal, $path:literal $(,)?) => {
        concat!(
            "[",
            $label,
            "]: ",
            env!("CARGO_MANIFEST_DIR"),
            "/screenshots/",
            $path,
        )
    };
}

#[cfg(docsrs)]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot_reference {
    ($label:literal, $path:literal $(,)?) => {
        concat!(
            "[", $label, "]: ",
            // The online path to the screenshot, on this released version.
            // Ideally, the "base URL" would be customisable, so end-users could use this macro too.x
            // The `v` is because of our tag name convention.
            "https://raw.githubusercontent.com/linebender/xilem/v", env!("CARGO_PKG_VERSION"), "/", env!("CARGO_PKG_NAME"), "/screenshots/", $path,
        )
    };
}
