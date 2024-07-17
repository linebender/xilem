// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for writing snapshot tests and comparing images.

use image::{GenericImageView as _, RgbImage};
use nv_flip::{FlipImageRgb8, DEFAULT_PIXELS_PER_DEGREE};

pub(crate) fn get_image_diff(ref_image: &RgbImage, new_image: &RgbImage) -> Option<RgbImage> {
    if ref_image.width() != new_image.width() || ref_image.height() != new_image.height() {
        panic!("Got a different size");
    }
    let ref_image_flip =
        FlipImageRgb8::with_data(ref_image.width(), ref_image.height(), &*ref_image);
    let new_image_flip =
        FlipImageRgb8::with_data(new_image.width(), new_image.height(), &*new_image);
    let error_map = nv_flip::flip(ref_image_flip, new_image_flip, DEFAULT_PIXELS_PER_DEGREE);
    let pool = nv_flip::FlipPool::from_image(&error_map);
    let mean = pool.mean();

    let mut is_changed = mean.abs() > 0.05;

    if !is_changed {
        return None;
    }

    let visualized = error_map.apply_color_lut(&nv_flip::magma_lut());
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

        if new_pixel != ref_pixel {
            is_changed = true;
            new_pixel
        } else {
            [0, 0, 0].into()
        }
    });

    if is_changed {
        Some(diff_image)
    } else {
        None
    }
}
