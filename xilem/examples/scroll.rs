// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing the capabilities of the Portal widget.

use masonry::properties::types::AsUnit;
use vello::peniko::color::AlphaColor;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, GridExt, MainAxisAlignment, flex_col, flex_row, grid, label, portal,
    sized_box, text_button,
};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;
use xilem_core::one_of::Either;

#[derive(Debug, Clone, Copy, Default)]
/// The layout mode we use.
enum BlocksLayout {
    /// A grid layout.
    ///
    /// We would like this to be the only supported option, but Grid currently
    /// completely ignores the size of its items, so it doesn't accurately show
    /// what we want it to show.
    Grid,
    /// A flex layout, using a flex for each row each
    /// inside a single flex column containing all the rows.
    #[default]
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
            blocks_layout: BlocksLayout::default(),
        }
    }
}

fn color_block(
    row_idx: i32,
    col_idx: i32,
    vertical_count: i32,
    horizontal_count: i32,
) -> impl WidgetView<Edit<AppState>> + use<> {
    let row_idx = row_idx as f32;
    let col_idx = col_idx as f32;
    let vertical_count = vertical_count as f32;
    let horizontal_count = horizontal_count as f32;

    sized_box(label(format!("r{}c{}", row_idx, col_idx)))
        .width(50.px())
        .height(50.px())
        .background_color(AlphaColor::new([
            row_idx / vertical_count,
            col_idx / horizontal_count,
            80. / 255.,
            1.0,
        ]))
}

fn grid_blocks(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
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

fn flex_blocks(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
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

fn blocks(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    match state.blocks_layout {
        BlocksLayout::Grid => Either::A(grid_blocks(state)),
        BlocksLayout::Flex => Either::B(flex_blocks(state)),
    }
}

fn app_logic(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    let blocks_layout_switch = flex_row((
        label("Switch layout:"),
        text_button(
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
        label("Vertical blocks (rows):"),
        text_button("-", |appstate: &mut AppState| appstate.vertical_count -= 1),
        text_button("+", |appstate: &mut AppState| appstate.vertical_count += 1),
    ));
    let horizontal_controls = flex_row((
        label("Horizontal blocks (columns):"),
        text_button("-", |appstate: &mut AppState| {
            appstate.horizontal_count -= 1;
        }),
        text_button("+", |appstate: &mut AppState| {
            appstate.horizontal_count += 1;
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
