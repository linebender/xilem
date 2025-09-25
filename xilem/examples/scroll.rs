// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing the capabilities of the Portal widget.

use masonry::properties::types::AsUnit;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, MainAxisAlignment, button, flex_col, flex_row, label, portal, sized_box,
};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem, palette};

struct AppState {
    vertical_count: usize,
    horizontal_count: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vertical_count: 30,
            horizontal_count: 15,
        }
    }
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let vertical_blocks = (0..state.vertical_count)
        .map(|row_idx| {
            let horizontal_blocks = (0..state.horizontal_count)
                .map(|col_idx| {
                    sized_box(label(format!("r{}c{}", row_idx, col_idx)))
                        .width(50.px())
                        .height(50.px())
                        .background_color(palette::css::WHITE.map_lightness(|l| {
                            l * (col_idx as f32) / (state.horizontal_count as f32)
                        }))
                })
                .collect::<Vec<_>>();
            flex_row(horizontal_blocks)
        })
        .collect::<Vec<_>>();

    let vertical_controls = flex_row((
        label("Vertical blocks"),
        button("+", |appstate: &mut AppState| appstate.vertical_count += 1),
        button("-", |appstate: &mut AppState| appstate.vertical_count -= 1),
    ));
    let horizontal_controls = flex_row((
        label("Horizontal blocks"),
        button("+", |appstate: &mut AppState| {
            appstate.horizontal_count += 1;
        }),
        button("-", |appstate: &mut AppState| {
            appstate.horizontal_count -= 1;
        }),
    ));

    let content = flex_col((vertical_controls, horizontal_controls, vertical_blocks))
        .main_axis_alignment(MainAxisAlignment::Start)
        .cross_axis_alignment(CrossAxisAlignment::Start);

    portal(content)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(AppState::default(), app_logic, WindowOptions::new("Scroll"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
