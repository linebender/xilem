// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(unused)]

use std::hint::black_box;

use xilem::WidgetView;
use xilem::core::Edit;
use xilem::core::ViewArgument;
use xilem::masonry::properties::types::AsUnit;
use xilem::masonry::util::debug_panic;
use xilem::palette::css;
use xilem::style::{Padding, Style};
use xilem::view::{
    CrossAxisAlignment, FlexExt, button, checkbox, flex, flex_col, flex_row, grid, image,
    indexed_stack, label, portal, progress_bar, prose, sized_box, slider, spinner, split, task,
    text_button, text_input, variable_label, virtual_scroll, worker, zstack,
};
use xilem_core::MessageResult;
use xilem_core::map_message;
use xilem_core::map_state;

fn widgets<State: ViewArgument + Send + Sync, const N: usize>()
-> impl WidgetView<State> + use<State, N> {
    map_message(
        flex_row((
            text_button("button", |_| todo!()),
            checkbox("checkbox", true, |_, _| todo!()),
            label("label"),
            portal(progress_bar(Some(0.))),
            prose("prose"),
            sized_box(slider(0., 0., 0., |_, _| todo!())),
            spinner(),
            split(
                text_input("input".into(), |_, _| todo!()),
                text_input("input".into(), |_, _| todo!()),
            ),
        )),
        |_, message: MessageResult<[u32; 0]>| MessageResult::Nop,
    )
}

#[cfg(feature = "compile-stress-test")]
fn mega_component() -> impl WidgetView<()> + use<> {
    flex_row((
        (
            widgets::<(), 00>(),
            widgets::<(), 01>(),
            widgets::<(), 02>(),
            widgets::<(), 03>(),
            widgets::<(), 04>(),
            widgets::<(), 05>(),
            widgets::<(), 06>(),
            widgets::<(), 07>(),
        ),
        (
            widgets::<(), 10>(),
            widgets::<(), 11>(),
            widgets::<(), 12>(),
            widgets::<(), 13>(),
            widgets::<(), 14>(),
            widgets::<(), 15>(),
            widgets::<(), 16>(),
            widgets::<(), 17>(),
        ),
        (
            widgets::<(), 20>(),
            widgets::<(), 21>(),
            widgets::<(), 22>(),
            widgets::<(), 23>(),
            widgets::<(), 24>(),
            widgets::<(), 25>(),
            widgets::<(), 26>(),
            widgets::<(), 27>(),
        ),
        (
            widgets::<(), 30>(),
            widgets::<(), 31>(),
            widgets::<(), 32>(),
            widgets::<(), 33>(),
            widgets::<(), 34>(),
            widgets::<(), 35>(),
            widgets::<(), 36>(),
            widgets::<(), 37>(),
        ),
        (
            widgets::<(), 40>(),
            widgets::<(), 41>(),
            widgets::<(), 42>(),
            widgets::<(), 43>(),
            widgets::<(), 44>(),
            widgets::<(), 45>(),
            widgets::<(), 46>(),
            widgets::<(), 47>(),
        ),
        (
            widgets::<(), 50>(),
            widgets::<(), 51>(),
            widgets::<(), 52>(),
            widgets::<(), 53>(),
            widgets::<(), 54>(),
            widgets::<(), 55>(),
            widgets::<(), 56>(),
            widgets::<(), 57>(),
        ),
        (
            widgets::<(), 60>(),
            widgets::<(), 61>(),
            widgets::<(), 62>(),
            widgets::<(), 63>(),
            widgets::<(), 64>(),
            widgets::<(), 65>(),
            widgets::<(), 66>(),
            widgets::<(), 67>(),
        ),
        (
            widgets::<(), 70>(),
            widgets::<(), 71>(),
            widgets::<(), 72>(),
            widgets::<(), 73>(),
            widgets::<(), 74>(),
            widgets::<(), 75>(),
            widgets::<(), 76>(),
            widgets::<(), 77>(),
        ),
    ))
}

#[cfg(not(feature = "compile-stress-test"))]
fn mega_component() -> impl WidgetView<()> + use<> {
    flex_row(widgets::<(), 0>())
}

fn main() {
    black_box(mega_component().boxed());
}
