// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label},
    MasonryView, Xilem,
};

struct AppState {
    count: u32,
}

impl AppState {
    fn decrement(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        };
    }
    fn increment(&mut self) {
        self.count += 1;
    }
}

fn app_logic(data: &mut AppState) -> impl MasonryView<AppState> {
    flex((
        button("-", AppState::decrement),
        label(format!("count: {}", data.count)),
        button("+", AppState::increment),
    ))
    .direction(xilem::Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let data = AppState { count: 0 };
    let app = Xilem::new(data, app_logic);
    app.run_windowed("Demo".into())?;
    Ok(())
}
