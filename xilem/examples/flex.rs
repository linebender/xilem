// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Flex properties can be set in Xilem.

use masonry::layout::AsUnit;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{FlexExt as _, FlexSpacer, Label, button, flex_row, label, sized_box};
use xilem::{EventLoop, TextAlign, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

/// A component to make a bigger than usual button.
fn big_button<F: Fn(&mut i32) + Send + Sync + 'static>(
    label: impl Into<Label>,
    callback: F,
) -> impl WidgetView<Edit<i32>> {
    // This being fully specified is "a known limitation of the trait solver"
    sized_box(button::<Edit<i32>, _, _, F>(label.into(), callback)).dims(40.px())
}

fn app_logic(data: &mut i32) -> impl WidgetView<Edit<i32>> + use<> {
    // This is the flex view, alternatives are `flex_col` or `flex` which allows dynamically switching the axis
    flex_row((
        FlexSpacer::Flex(1.0),
        big_button("-", |data| {
            *data -= 1;
        }),
        FlexSpacer::Fixed(30.px()),
        label(format!("count: {data}"))
            .text_size(32.)
            .text_alignment(TextAlign::Center)
            .flex(5.0),
        FlexSpacer::Fixed(30.px()),
        big_button("+", |data| {
            *data += 1;
        }),
        FlexSpacer::Flex(1.0),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(0, app_logic, WindowOptions::new("Centered Flex"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
