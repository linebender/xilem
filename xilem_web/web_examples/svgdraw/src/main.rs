// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showing how SVG paths can be used for a vector-drawing application

use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;
use xilem_web::{
    document_body,
    elements::{
        html::{div, input, label, span},
        svg::{g, svg},
    },
    input_event_target_value,
    interfaces::{Element, SvgGeometryElement, SvgPathElement, SvggElement},
    modifiers::style as s,
    svg::{
        kurbo::{BezPath, Point, QuadSpline, Shape, Stroke},
        peniko::Color,
    },
    AnyDomView, App, DomFragment, PointerMsg,
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
    #![allow(
        clippy::cast_possible_truncation,
        reason = "This will never happen here"
    )]
    RAINBOW_COLORS[(web_sys::js_sys::Math::random() * 1000000.0) as usize % RAINBOW_COLORS.len()]
}

struct SplineLine {
    points: Vec<Point>,
    color: Color,
    width: f64,
}

impl SplineLine {
    fn new(p: Point, color: Color, width: f64) -> Self {
        Self {
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
    active_line: Option<SplineLine>,
    cursor_position: Point,
    canvas_position: Point,
    draw_position: Point,
    lines_cached: Vec<Rc<AnyDomView<Draw>>>,
    new_line_width: f64,
    is_moving: bool,
    zoom: f64,
}

impl Draw {
    fn update_cursor(&mut self, cursor: Point) {
        let last_cursor_position = self.cursor_position;
        self.cursor_position = cursor;
        let cursor_delta = self.cursor_position - last_cursor_position;
        let zoom_corrected_delta = cursor_delta / self.zoom;
        self.draw_position += zoom_corrected_delta;
        if self.is_moving {
            self.canvas_position -= zoom_corrected_delta;
            self.draw_position -= zoom_corrected_delta;
        }
    }

    // TODO support pinch as well
    fn zoom_with_wheel_event(&mut self, event: web_sys::WheelEvent) {
        let delta_mode_factor = match event.delta_mode() {
            2 => 100.0, // Pages
            1 => 16.0,  // Lines
            _ => 1.0,   // Pixels and Default
        };
        let mut delta = event.delta_y() * delta_mode_factor;

        let delta_sign = delta.signum();
        let factor = 1.0 + 0.001 * delta.abs();
        delta = if delta_sign < 0.0 {
            factor
        } else {
            1.0 / factor
        };

        self.zoom *= delta;
        self.canvas_position -= (self.cursor_position.to_vec2() * (1.0 - delta)) / self.zoom;
    }

    fn start_new_line(&mut self) {
        let line = SplineLine::new(self.draw_position, random_color(), self.new_line_width);
        self.lines_cached
            .push(Rc::new(line.view()) as Rc<AnyDomView<Self>>);
        self.active_line = Some(line);
    }

    fn extend_active_line(&mut self) {
        if let Some(cur_line) = &mut self.active_line {
            cur_line.points.push(self.draw_position);
            *self.lines_cached.last_mut().unwrap() =
                Rc::new(cur_line.view()) as Rc<AnyDomView<Self>>;
        }
    }

    fn finish_active_line(&mut self) {
        self.active_line = None;
    }

    fn view(&mut self) -> impl DomFragment<Self> {
        let x = -self.canvas_position.x;
        let y = -self.canvas_position.y;
        let zoom = self.zoom;
        let canvas = svg(g(self.lines_cached.clone())
            .fill(Color::TRANSPARENT)
            .style(s(
                "transform",
                format!("scale({zoom}) translate({x}px, {y}px)"),
            )))
        .pointer(|state: &mut Self, event| {
            state.update_cursor(event.position());
            match event {
                PointerMsg::Down(event) => match event.button {
                    0 => state.start_new_line(),
                    1 | 2 => state.is_moving = true,
                    _ => (),
                },
                PointerMsg::Move(_) => state.extend_active_line(),
                PointerMsg::Up(event) => match event.button {
                    0 => state.finish_active_line(),
                    1 | 2 => state.is_moving = false,
                    _ => (),
                },
            };
        })
        .style([s("width", "100vw"), s("height", "100vh")])
        .on_wheel(|state, event| state.zoom_with_wheel_event(event))
        .on_click(|_, event| event.prevent_default())
        .passive(false)
        .on_contextmenu(|_, event| event.prevent_default())
        .passive(false);

        let controls = label((
            // a space width would be more ideal, but for some reason spaces are truncated...
            span(format!("Stroke width {:>05.2}: ", self.new_line_width)),
            div(input(())
                .attr("type", "range")
                .attr("min", 0.1)
                .attr("max", 30)
                .attr("step", 0.01)
                .attr("value", self.new_line_width)
                .on_input(|state: &mut Self, event| {
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

fn main() {
    console_error_panic_hook::set_once();
    App::new(
        document_body(),
        Draw {
            new_line_width: 5.0,
            zoom: 1.0,
            ..Draw::default()
        },
        Draw::view,
    )
    .run();
}
