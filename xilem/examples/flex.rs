// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Flex properties can be set in Xilem.

use masonry::properties::types::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::view::{FlexExt as _, FlexSpacer, Label, button, flex_row, label, sized_box};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

/// A component to make a bigger than usual button
fn big_button(
    label: impl Into<Label>,
    callback: impl Fn(&mut i32) + Send + Sync + 'static,
) -> impl WidgetView<i32> {
    sized_box(button(label, callback))
        .width(Length::px(40.))
        .height(Length::px(40.))
}

fn app_logic(data: &mut i32) -> impl WidgetView<i32> + use<> {
    flex_row((
        FlexSpacer::Fixed(Length::px(30.0)),
        big_button("-", |data| {
            *data -= 1;
        }),
        FlexSpacer::Flex(1.0),
        label(format!("count: {data}")).text_size(32.).flex(5.0),
        FlexSpacer::Flex(1.0),
        big_button("+", |data| {
            *data += 1;
        }),
        FlexSpacer::Fixed(Length::px(30.0)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(0, app_logic, WindowOptions::new("Centered Flex"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
