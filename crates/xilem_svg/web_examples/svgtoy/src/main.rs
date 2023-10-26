// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use xilem_svg::{
    group,
    kurbo::{self, Rect},
    App, PointerMsg, View, ViewExt,
};

#[derive(Default)]
struct AppState {
    x: f64,
    y: f64,
    grab: GrabState,
}

#[derive(Default)]
struct GrabState {
    is_down: bool,
    id: i32,
    dx: f64,
    dy: f64,
}

impl GrabState {
    fn handle(&mut self, x: &mut f64, y: &mut f64, p: &PointerMsg) {
        match p {
            PointerMsg::Down(e) => {
                if e.button == 0 {
                    self.dx = *x - e.x;
                    self.dy = *y - e.y;
                    self.id = e.id;
                    self.is_down = true;
                }
            }
            PointerMsg::Move(e) => {
                if self.is_down && self.id == e.id {
                    *x = self.dx + e.x;
                    *y = self.dy + e.y;
                }
            }
            PointerMsg::Up(e) => {
                if self.id == e.id {
                    self.is_down = false;
                }
            }
        }
    }
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    let v = (0..10)
        .map(|i| Rect::from_origin_size((10.0 * i as f64, 150.0), (8.0, 8.0)))
        .collect::<Vec<_>>();
    group((
        Rect::new(100.0, 100.0, 200.0, 200.0).clicked(|_| {
            web_sys::console::log_1(&"app logic clicked".into());
        }),
        Rect::new(210.0, 100.0, 310.0, 200.0),
        Rect::new(320.0, 100.0, 420.0, 200.0).class("red"),
        Rect::new(state.x, state.y, state.x + 100., state.y + 100.)
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.x, &mut s.y, &msg)),
        group(v),
        Rect::new(210.0, 210.0, 310.0, 310.0).pointer(|_, e| {
            web_sys::console::log_1(&format!("pointer event {e:?}").into());
        }),
        kurbo::Line::new((310.0, 210.0), (410.0, 310.0)),
        kurbo::Circle::new((460.0, 260.0), 45.0).clicked(|_| {
            web_sys::console::log_1(&"circle clicked".into());
        }),
    ))
    //button(format!("Count {}", count), |count| *count += 1)
}

pub fn main() {
    console_error_panic_hook::set_once();
    let app = App::new(AppState::default(), app_logic);
    app.run();
}
