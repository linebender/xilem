// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Showcase a trasnsparent window.

use masonry::layout::AsUnit;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::*;
use xilem::{Color, EventLoop, WindowId, WindowView, Xilem};

#[derive(Debug, Clone)]
struct AppState {
    alpha: f32,

    window_id: WindowId,
    is_running: bool,
}

impl xilem::AppState for AppState {
    fn keep_running(&self) -> bool {
        self.is_running
    }
}

fn app_logic(state: &mut AppState) -> impl Iterator<Item = WindowView<AppState>> + use<> {
    let base_color = Color::new([0., 0., 0., state.alpha]);

    let root_view = flex_col((
        FlexSpacer::Flex(1.),
        flex_row((
            text_button("-", |state: &mut AppState| {
                state.alpha = (state.alpha - 0.25).max(0.);
            }),
            text_button("+", |state: &mut AppState| {
                state.alpha = (state.alpha + 0.25).min(1.);
            }),
        ))
        .gap(10.px()),
        FlexSpacer::Flex(1.),
    ))
    .padding(20.);

    std::iter::once(
        xilem::window(state.window_id, "Transparency Demo", root_view)
            .with_base_color(base_color)
            .with_options(|o| {
                o.with_transparent(true)
                    .on_close(|state: &mut AppState| state.is_running = false)
            }),
    )
}

fn main() -> Result<(), EventLoopError> {
    Xilem::new(
        AppState {
            window_id: WindowId::next(),
            is_running: true,
            alpha: 0.5,
        },
        app_logic,
    )
    .run_in(EventLoop::with_user_event())
}
