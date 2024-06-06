// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label},
    EventLoop, WidgetView, Xilem,
};

fn app_logic(data: &mut i32) -> impl WidgetView<i32> {
    flex((
        button("-", |data| {
            *data -= 1;
        }),
        label(format!("count: {}", data)),
        button("+", |data| {
            *data += 1;
        }),
    ))
    .direction(xilem::Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(0, app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Centered Flex".into())?;
    Ok(())
}
