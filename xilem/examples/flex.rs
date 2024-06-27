// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    widget::{CrossAxisAlignment, MainAxisAlignment},
    ArcStr,
};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label, sized_box},
    EventLoop, WidgetView, Xilem,
};

/// A component to make a bigger than usual button
fn big_button(
    label: impl Into<ArcStr>,
    callback: impl Fn(&mut i32) + Send + Sync + 'static,
) -> impl WidgetView<i32> {
    sized_box(button(label, callback)).width(40.).height(40.)
}

fn app_logic(data: &mut i32) -> impl WidgetView<i32> {
    flex((
        big_button("-", |data| {
            *data -= 1;
        }),
        label(format!("count: {}", data)).text_size(32.),
        big_button("+", |data| {
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
