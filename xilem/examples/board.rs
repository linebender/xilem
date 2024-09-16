// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    widget::{CrossAxisAlignment, MainAxisAlignment},
    Size,
};
use winit::error::EventLoopError;
use xilem::view::{
    board, button, flex, label, Axis, BoardExt, BoardParams, FlexExt as _, FlexSpacer, ShapeExt,
};
use xilem::{Color, EventLoop, WidgetView, Xilem};

struct AppState {
    buttons: Vec<bool>,
    clicked: Option<usize>,
}

impl AppState {
    fn view(&mut self) -> impl WidgetView<Self> {
        flex((
            FlexSpacer::Fixed(30.0),
            flex((
                button("B", |state: &mut AppState| state.buttons.push(true)),
                button("C", |state: &mut AppState| state.buttons.push(false)),
                button("-", |state: &mut AppState| {
                    state.buttons.pop();
                    state.clicked = None;
                }),
                label(self.clicked.map_or_else(
                    || String::from("Nothing has been clicked."),
                    |i| format!("Button {i} has been clicked."),
                )),
            ))
            .direction(Axis::Horizontal),
            FlexSpacer::Fixed(10.0),
            board(
                self.buttons
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, is_button)| {
                        let origin = i as f64 * 15. + 10.;
                        let size = Size::new(30., 30.);
                        if is_button {
                            button(i.to_string(), move |state: &mut AppState| {
                                state.clicked = Some(i);
                            })
                            .positioned(BoardParams::new((origin, origin), size))
                            .into_any_board()
                        } else {
                            vello::kurbo::Circle::new((origin + 15., origin + 15.), 15.)
                                .view()
                                .fill_brush(Color::NAVY)
                                .stroke_brush(Color::PAPAYA_WHIP)
                                .stroke_style(vello::kurbo::Stroke::new(2.))
                                .into_any_board()
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .flex(1.),
        ))
        .direction(Axis::Vertical)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
    }
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(
        AppState {
            buttons: Vec::new(),
            clicked: None,
        },
        AppState::view,
    );
    app.run_windowed(EventLoop::with_user_event(), "Board".into())?;
    Ok(())
}
