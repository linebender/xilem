// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Stress test for the build speed a Xilem component with lots of styles applied.

#![allow(unused)]

use std::hint::black_box;

use xilem::WidgetView;
use xilem::core::{MessageResult, ViewArgument, map_message_result};
use xilem::style::Style as _;
use xilem::view::{
    checkbox, flex_row, label, portal, progress_bar, prose, sized_box, slider, spinner, split,
    text_button, text_input,
};

#[cfg(compile_stress_test)]
fn prop_stack() -> impl WidgetView<()> + use<> {
    prose("")
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
        .border_width(0.5)
}

#[cfg(not(compile_stress_test))]
fn prop_stack() -> impl WidgetView<()> + use<> {
    prose("")
        // We use only one to check it compiles
        .border_width(0.5)
}

#[test]
fn test() {
    black_box(prop_stack().boxed());
}
