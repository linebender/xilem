// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::UnwrapThrowExt;
use xilem_web::{
    document_body,
    elements::{
        html::{div, input, label, span},
        svg::{g, svg},
    },
    input_event_target_value,
    interfaces::*,
    modifiers::style as s,
    svg::{
        kurbo::{BezPath, Point, QuadSpline, Shape, Stroke},
        peniko::Color,
    },
    App, DomFragment, PointerMsg,
};

const RAINBOW_COLORS: &[Color] = &[
    Color::rgb8(228, 3, 3),     // Red
    Color::rgb8(255, 140, 0),   // Orange
    Color::rgb8(255, 237, 0),   // Yellow
    Color::rgb8(0, 128, 38),    // Green
    Color::rgb8(0, 76, 255),    // Indigo
    Color::rgb8(115, 41, 130),  // Violet
    Color::rgb8(214, 2, 112),   // Pink
    Color::rgb8(155, 79, 150),  // Lavender
    Color::rgb8(0, 56, 168),    // Blue
    Color::rgb8(91, 206, 250),  // Light Blue
    Color::rgb8(245, 169, 184), // Pink
];

fn random_color() -> Color {
    RAINBOW_COLORS[(web_sys::js_sys::Math::random() * 1000000.0) as usize % RAINBOW_COLORS.len()]
}

struct SplineLine {
    points: Vec<Point>,
    color: Color,
    width: f64,
}

impl SplineLine {
    fn new(p: Point, color: Color, width: f64) -> Self {
        SplineLine {
            points: vec![p],
            color,
            width,
        }
    }

    fn view<State: 'static>(&self) -> impl SvgPathElement<State> {
        QuadSpline::new(self.points.clone())
            .to_quads()
            .fold(BezPath::new(), |mut b, q| {
                b.extend(q.path_elements(0.0));
                b
            })
            .stroke(self.color, Stroke::new(self.width))
    }
}

#[derive(Default)]
struct Draw {
    lines: Vec<SplineLine>,
    new_line_width: f64,
    is_drawing: bool,
}

impl Draw {
    fn view(&mut self) -> impl DomFragment<Draw> {
        let lines = self.lines.iter().map(SplineLine::view).collect::<Vec<_>>();
        let canvas = svg(g(lines).fill(Color::TRANSPARENT))
            .pointer(|state: &mut Draw, e| {
                match e {
                    PointerMsg::Down(p) => {
                        let l = SplineLine::new(p.position, random_color(), state.new_line_width);
                        state.lines.push(l);
                        state.is_drawing = true;
                    }
                    PointerMsg::Move(p) => {
                        if state.is_drawing {
                            state.lines.last_mut().unwrap().points.push(p.position);
                        }
                    }
                    PointerMsg::Up(_) => state.is_drawing = false,
                };
            })
            .style([s("width", "100vw"), s("height", "100vh")]);

        let controls = label((
            span("Stroke width:"),
            div(input(())
                .attr("type", "range")
                .attr("min", 1)
                .attr("max", 30)
                .attr("step", 0.01)
                .attr("value", self.new_line_width)
                .on_input(|state: &mut Draw, event| {
                    state.new_line_width = input_event_target_value(&event)
                        .unwrap_throw()
                        .parse()
                        .unwrap_throw();
                }))
            .class("value-range"),
        ))
        .class("controls");
        (controls, canvas)
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(
        document_body(),
        Draw {
            new_line_width: 5.0,
            ..Draw::default()
        },
        Draw::view,
    )
    .run();
}
