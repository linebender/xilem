// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Helper functions for writing snapshot tests and comparing images.

use image::{GenericImageView as _, RgbaImage};

use crate::piet::{BitmapTarget, ImageFormat};
use crate::Size;

pub(crate) fn get_rgba_image(render_target: &mut BitmapTarget, window_size: Size) -> RgbaImage {
    let pixels = render_target
        .to_image_buf(ImageFormat::RgbaPremul)
        .unwrap()
        .raw_pixels_shared();

    RgbaImage::from_raw(
        window_size.width as u32,
        window_size.height as u32,
        Vec::from(pixels.as_ref()),
    )
    .unwrap()
}

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
