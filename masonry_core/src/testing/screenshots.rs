// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for writing snapshot tests and comparing images.

use image::{GenericImageView as _, Pixel as _, Rgb, RgbImage};

#[cfg(docsrs)]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        concat!(
            "![", $($caption,)? "]",
            "(", "https://media.githubusercontent.com/media/linebender/xilem/",
            "masonry-v", env!("CARGO_PKG_VERSION"), "/masonry_core/src/", $path,
            ")",
        )
    };
}

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

// Copy-pasted from kompari
fn pixel_min_max_distance(left: Rgb<u8>, right: Rgb<u8>) -> (u8, u8) {
    left.channels()
        .iter()
        .zip(right.channels())
        .fold((0, 0), |(min, max), (c1, c2)| {
            if c2 > c1 {
                (min, max.max(c2 - c1))
            } else {
                (min.max(c1 - c2), max)
            }
        })
}

pub(crate) fn get_image_diff(ref_image: &RgbImage, new_image: &RgbImage) -> Option<RgbImage> {
    // TODO - Handle this case more gracefully.
    assert_eq!(
        (ref_image.width(), ref_image.height()),
        (new_image.width(), new_image.height()),
        "New image (right) has different size from old image (left)."
    );

    let mut max_distance: u32 = 0;
    for (p1, p2) in ref_image.pixels().zip(new_image.pixels()) {
        let (diff_min, diff_max) = pixel_min_max_distance(*p1, *p2);
        let new_max = std::cmp::max(diff_max, diff_min) as u32;
        max_distance = std::cmp::max(max_distance, new_max);
    }

    const EXPECTED_MAX_DISTANCE: u32 = 16;
    if max_distance <= EXPECTED_MAX_DISTANCE {
        return None;
    }

    let width = std::cmp::max(ref_image.width(), new_image.width());
    let height = std::cmp::max(ref_image.height(), new_image.height());

    let diff_image = RgbImage::from_fn(width, height, |x, y| {
        let ref_pixel = if ref_image.in_bounds(x, y) {
            *ref_image.get_pixel(x, y)
        } else {
            [0, 0, 0].into()
        };
        let new_pixel = if new_image.in_bounds(x, y) {
            *new_image.get_pixel(x, y)
        } else {
            [255, 255, 255].into()
        };

        let (diff_min, diff_max) = pixel_min_max_distance(ref_pixel, new_pixel);
        let diff_abs = std::cmp::max(diff_min, diff_max);

        if diff_abs as u32 > EXPECTED_MAX_DISTANCE {
            new_pixel
        } else {
            [0, 0, 0].into()
        }
    });

    Some(diff_image)
}
