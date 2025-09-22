// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Showcase a trasnsparent window.

use masonry::properties::types::AsUnit;
use winit::error::EventLoopError;
use xilem::{Color, EventLoop, WindowId, WindowView, Xilem, style::Style, view::*};

#[derive(Debug, Clone)]
struct AppState {
    opacity_level: u8,

    window_id: WindowId,
    is_running: bool,
}

impl xilem::AppState for AppState {
    fn keep_running(&self) -> bool {
        self.is_running
    }
}

fn app_logic(state: &mut AppState) -> impl Iterator<Item = WindowView<AppState>> + use<> {
    let base_color = Color::new([0., 0., 0., state.opacity_level as f32 / 4.]);

    let root_view = flex_col((
        FlexSpacer::Flex(1.),
        flex_row((
            button("-", |state: &mut AppState| {
                state.opacity_level = state.opacity_level.saturating_sub(1);
            }),
            button("+", |state: &mut AppState| {
                state.opacity_level = (state.opacity_level + 1).min(4);
            }),
        ))
        .gap(10.px()),
        FlexSpacer::Flex(1.),
    ))
    .padding(20.);

    std::iter::once(
        xilem::window(state.window_id, "Transparency Demo", root_view)
            .with_base_color(base_color)
            .with_options(|o| o.with_transparent(true)),
    )
}

fn main() -> Result<(), EventLoopError> {
    Xilem::new(
        AppState {
            window_id: WindowId::next(),
            is_running: true,
            opacity_level: 2,
        },
        app_logic,
    )
    .run_in(EventLoop::with_user_event())
}
