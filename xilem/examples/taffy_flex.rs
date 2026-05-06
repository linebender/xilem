// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Flex properties can be set in Xilem.

use masonry::layout::AsUnit;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{Label, TaffyExt, TaffySpacer, button, label, taffy_row};
use xilem::{EventLoop, TextAlign, WidgetView, WindowOptions, Xilem};

/// A component to make a bigger than usual button.
fn big_button<F: Fn(&mut i32) + Send + Sync + 'static>(
    label: impl Into<Label>,
    callback: F,
) -> impl WidgetView<i32> {
    button(label.into(), callback).dims(40.px())
}

fn app_logic(data: &mut i32) -> impl WidgetView<i32> + use<> {
    // This is the flex view, alternatives are `flex_col` or `flex` which allows dynamically switching the axis
    taffy_row((
        TaffySpacer::flex(1.0),
        big_button("-", |data| {
            *data -= 1;
        }),
        TaffySpacer::fixed(30.px()),
        label(format!("count: {data}"))
            .text_size(32.)
            .text_alignment(TextAlign::Center)
            .grow(5.0),
        TaffySpacer::fixed(30.px()),
        big_button("+", |data| {
            *data += 1;
        }),
        TaffySpacer::flex(1.0),
    ))
    .align_items(masonry::widgets::taffy::AlignItems::Center)
    .gap(10.px())
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(0, app_logic, WindowOptions::new("Centered Taffy Flex"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
