// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_web::{
    document_body,
    elements::svg::{g, svg, text},
    interfaces::*,
    style as s,
    svg::{
        kurbo::{Circle, Line, Rect, Stroke},
        peniko::Color,
    },
    App, DomView, PointerMsg,
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

fn app_logic(state: &mut AppState) -> impl DomView<AppState> {
    let v = (0..10)
        .map(|i| {
            Rect::from_origin_size((10.0 * i as f64, 150.0), (8.0, 8.0))
                .rotate(0.003 * (i as f64) * state.x)
        })
        .collect::<Vec<_>>();
    svg(g((
        Rect::new(100.0, 100.0, 200.0, 200.0).on_click(|_, _| {
            web_sys::console::log_1(&"app logic clicked".into());
        }),
        Rect::new(210.0, 100.0, 310.0, 200.0)
            .fill(Color::LIGHT_GRAY)
            .stroke(Color::BLUE, Default::default())
            .scale((state.x / 100.0 + 1.0, state.y / 100.0 + 1.0)),
        Rect::new(320.0, 100.0, 420.0, 200.0).class("red"),
        Rect::new(state.x, state.y, state.x + 100., state.y + 100.)
            .fill(Color::rgba8(100, 100, 255, 100))
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.x, &mut s.y, &msg)),
        text("drag me around")
            .style(s(
                "transform",
                format!("translate({}px, {}px)", state.x, state.y + 50.0),
            ))
            .style([s("font-size", "10px"), s("pointer-events", "none")]),
        g(v).style(s("transform", "translate(430px, 0)")) // untyped transform can be combined with transform modifiers, though this overwrites previously set `transform` values
            .scale(state.y / 100.0 + 1.0),
        Rect::new(210.0, 210.0, 310.0, 310.0).pointer(|_, e| {
            web_sys::console::log_1(&format!("pointer event {e:?}").into());
        }),
        Line::new((310.0, 210.0), (410.0, 310.0)).stroke(
            Color::YELLOW_GREEN,
            Stroke::new(1.0).with_dashes(state.x, [7.0, 1.0]),
        ),
        Circle::new((460.0, 260.0), 45.0).on_click(|_, _| {
            web_sys::console::log_1(&"circle clicked".into());
        }),
    )))
    .attr("width", 800)
    .attr("height", 600)
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
