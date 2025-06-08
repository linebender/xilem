// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This showcase demonstrates how to use the image widget and its
//! properties. You can change the parameters in the GUI to see how
//! everything behaves.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::{Action, ObjectFit, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::widgets::{Image, RootWidget};
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use vello::peniko::{Image as ImageBuf, ImageFormat};
use winit::window::Window;

struct Driver;

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        _window_id: WindowId,
        _ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        _action: Action,
    ) {
    }
}

fn make_image() -> Image {
    let image_bytes = include_bytes!("./assets/PicWithAlpha.png");
    let image_data = image::load_from_memory(image_bytes).unwrap().to_rgba8();
    let (width, height) = image_data.dimensions();
    let png_data = ImageBuf::new(
        image_data.to_vec().into(),
        ImageFormat::Rgba8,
        width,
        height,
    );

    Image::new(png_data).fit_mode(ObjectFit::Contain)
}

fn main() {
    let window_size = LogicalSize::new(650.0, 450.0);
    let window_attributes = Window::default_attributes()
        .with_title("Simple image example")
        .with_min_inner_size(window_size)
        .with_max_inner_size(window_size);

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![(
            WindowId::next(),
            window_attributes,
            Box::new(RootWidget::new(make_image())),
        )],
        Driver,
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;
    use masonry::theme::default_property_set;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(default_property_set(), make_image());
        assert_render_snapshot!(harness, "example_simple_image_initial");
    }
}
