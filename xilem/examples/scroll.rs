// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing the capabilities of the Portal widget.

use masonry::properties::types::AsUnit;
use vello::peniko::color::AlphaColor;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, GridExt, MainAxisAlignment, button, flex_col, flex_row, grid, label,
    portal, sized_box,
};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::one_of::Either;

#[derive(Debug, Clone, Copy)]
enum BlocksLayout {
    Grid,
    Flex,
}

struct AppState {
    vertical_count: i32,
    horizontal_count: i32,
    blocks_layout: BlocksLayout,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vertical_count: 30,
            horizontal_count: 30,
            blocks_layout: BlocksLayout::Flex,
        }
    }
}

fn color_block(
    row_idx: i32,
    col_idx: i32,
    vertical_count: i32,
    horizontal_count: i32,
) -> impl WidgetView<AppState> + use<> {
    let row_idx = row_idx as f32;
    let col_idx = col_idx as f32;
    let vertical_count = vertical_count as f32;
    let horizontal_count = horizontal_count as f32;

    sized_box(label(format!("r{}c{}", row_idx, col_idx)))
        .width(50.px())
        .height(50.px())
        .background_color(AlphaColor::from_rgb8(
            (row_idx / vertical_count * 255.).round().clamp(0., 255.) as u8,
            (col_idx / horizontal_count * 255.).round().clamp(0., 255.) as u8,
            80,
        ))
}

fn grid_blocks(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let (vertical_count, horizontal_count) = (state.vertical_count, state.horizontal_count);

    grid(
        (0..vertical_count)
            .flat_map(|row_idx| {
                (0..horizontal_count).map(move |col_idx| {
                    color_block(row_idx, col_idx, vertical_count, horizontal_count)
                        .grid_pos(row_idx, col_idx)
                })
            })
            .collect::<Vec<_>>(),
        horizontal_count,
        vertical_count,
    )
    .spacing(10.px())
}

fn flex_blocks(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let (vertical_count, horizontal_count) = (state.vertical_count, state.horizontal_count);

    flex_col(
        (0..vertical_count)
            .map(|row_idx| {
                flex_row(
                    (0..horizontal_count)
                        .map(|col_idx| {
                            color_block(row_idx, col_idx, vertical_count, horizontal_count)
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn blocks(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    match state.blocks_layout {
        BlocksLayout::Grid => Either::A(grid_blocks(state)),
        BlocksLayout::Flex => Either::B(flex_blocks(state)),
    }
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let blocks_layout_switch = flex_row((
        label("Switch layout"),
        button(
            format!("{:?}", state.blocks_layout),
            |state: &mut AppState| {
                let next = match state.blocks_layout {
                    BlocksLayout::Grid => BlocksLayout::Flex,
                    BlocksLayout::Flex => BlocksLayout::Grid,
                };
                state.blocks_layout = next;
            },
        ),
    ));
    let vertical_controls = flex_row((
        label("Vertical(Row) blocks"),
        button("+", |appstate: &mut AppState| appstate.vertical_count += 1),
        button("-", |appstate: &mut AppState| appstate.vertical_count -= 1),
    ));
    let horizontal_controls = flex_row((
        label("Horizontal(Column) blocks"),
        button("+", |appstate: &mut AppState| {
            appstate.horizontal_count += 1;
        }),
        button("-", |appstate: &mut AppState| {
            appstate.horizontal_count -= 1;
        }),
    ));

    let content = flex_col((
        blocks_layout_switch,
        vertical_controls,
        horizontal_controls,
        blocks(state),
    ))
    .main_axis_alignment(MainAxisAlignment::Start)
    .cross_axis_alignment(CrossAxisAlignment::Start);

    portal(content)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(AppState::default(), app_logic, WindowOptions::new("Scroll"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
