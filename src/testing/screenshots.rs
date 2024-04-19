// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Helper functions for writing snapshot tests and comparing images.

use image::{GenericImageView as _, RgbaImage};

pub(crate) fn get_image_diff(ref_image: &RgbaImage, new_image: &RgbaImage) -> Option<RgbaImage> {
    let mut is_changed = false;

    if ref_image.width() != new_image.width() || ref_image.height() != new_image.height() {
        is_changed = true;
    }

    let width = std::cmp::max(ref_image.width(), new_image.width());
    let height = std::cmp::max(ref_image.height(), new_image.height());

    let diff_image = RgbaImage::from_fn(width, height, |x, y| {
        let ref_pixel = if ref_image.in_bounds(x, y) {
            *ref_image.get_pixel(x, y)
        } else {
            [0, 0, 0, 0].into()
        };
        let new_pixel = if new_image.in_bounds(x, y) {
            *new_image.get_pixel(x, y)
        } else {
            [255, 255, 255, 255].into()
        };

        if new_pixel != ref_pixel {
            is_changed = true;
            new_pixel
        } else {
            [0, 0, 0, 0].into()
        }
    });

    if is_changed {
        Some(diff_image)
    } else {
        None
    }
}
