// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use taffy::Display::Flex;
use taffy::{AlignItems, JustifyContent};
use masonry::{
    ArcStr,
};
use winit::error::EventLoopError;
use xilem::{
    view::{button, label, sized_box},
    EventLoop, WidgetView, Xilem,
};
use xilem::view::taffy_layout;

/// A component to make a bigger than usual button
fn big_button(
    label: impl Into<ArcStr>,
    callback: impl Fn(&mut i32) + Send + Sync + 'static,
) -> impl WidgetView<i32> {
    sized_box(button(label, callback)).width(40.).height(40.)
}

fn app_logic(data: &mut i32) -> impl WidgetView<i32> {
    taffy_layout((
        big_button("-", |data| {
            *data -= 1;
        }),
        label(format!("count: {}", data)).text_size(32.),
        big_button("+", |data| {
            *data += 1;
        }),
    ), taffy::Style{
        display: Flex,
        justify_content: Option::from(JustifyContent::Center),
        align_items: Option::from(AlignItems::Center),
        gap: taffy::Size{
            width: taffy::LengthPercentage::Length(15.0),
            height: taffy::LengthPercentage::Length(15.0),
        },
        ..taffy::Style::default()
    })
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(0, app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Centered Flex with Taffy".into())?;
    Ok(())
}
