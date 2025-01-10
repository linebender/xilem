// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::Label;
use vello::kurbo::Vec2;
use vello::peniko::Color;
use xilem::{view::SizedBox, App, AppLauncher, View};

fn app_logic() -> impl View<()> {
    SizedBox::new(Label::new("Hello Shadow!"))
        .width(200.0)
        .height(100.0)
        .background(Color::rgb8(255, 255, 255))
        .rounded(10.0)
        .shadow(
            Color::rgba8(0, 0, 0, 128),
            Vec2::new(5.0, 5.0),
            10.0,
            0.0,
            Some(10.0),
        )
        .padding(20.0)
}

pub fn main() {
    let app = App::new((), app_logic);
    AppLauncher::new(app)
        .use_simple_logger()
        .launch()
        .expect("Failed to launch app");
}
