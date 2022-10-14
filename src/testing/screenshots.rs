use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use druid_shell::{KeyEvent, Modifiers, MouseButton, MouseButtons};
pub use druid_shell::{
    RawMods, Region, Scalable, Scale, Screen, SysMods, TimerToken, WindowHandle, WindowLevel,
    WindowState,
};
use image::io::Reader as ImageReader;
use image::{GenericImageView as _, RgbaImage};

//use crate::ext_event::ExtEventHost;
use crate::command::CommandQueue;
use crate::debug_logger::DebugLogger;
use crate::ext_event::ExtEventQueue;
use crate::piet::{BitmapTarget, Device, Error, ImageFormat, Piet};
use crate::platform::PendingWindow;
use crate::widget::WidgetRef;
use crate::widget::WidgetState;
use crate::*;

pub fn get_rgba_image(render_target: &mut BitmapTarget, window_size: Size) -> RgbaImage {
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

pub fn get_image_diff(ref_image: &RgbaImage, new_image: &RgbaImage) -> Option<RgbaImage> {
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
