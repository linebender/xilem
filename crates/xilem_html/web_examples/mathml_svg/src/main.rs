use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_html::{
    document_body, elements::html, elements::mathml as ml, elements::svg, interfaces::*, App,
};

struct Triangle {
    a: u32,
    b: u32,
}

fn get_value(event: web_sys::Event) -> u32 {
    event
        .target()
        .unwrap_throw()
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap_throw()
        .value_as_number() as u32
}

fn label(
    l: impl ToString,
    x: u32,
    y: u32,
    dy: &'static str,
    anchor: &'static str,
) -> impl SvgTextElement<Triangle> {
    svg::text(l.to_string())
        .attr("x", x)
        .attr("y", y)
        .attr("dy", dy)
        .attr("text-anchor", anchor)
}

fn slider(
    max: u32,
    value: u32,
    cb: fn(&mut Triangle, web_sys::Event),
) -> impl HtmlInputElement<Triangle> {
    html::input(())
        .attr("type", "range")
        .attr("min", 1)
        .attr("max", max)
        .attr("value", value)
        .on_input(cb)
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(Triangle { a: 200, b: 100 }, |t| {
        let x1 = 390;
        let y1 = 30;
        let x2 = x1;
        let y2 = 30 + t.b;
        let x3 = x1 - t.a;
        let y3 = 30 + t.b;

        let a_label_x = (x1 + x3) / 2;
        let b_label_y = (y2 + y1) / 2;
        let c = ((t.a * t.a + t.b * t.b) as f32).sqrt();

        html::div((
            html::h1("Pythagorean theorem"),
            svg::svg((
                svg::polygon(())
                    .attr("points", format!("{x1},{y1} {x2},{y2} {x3},{y3}"))
                    .attr("style", "fill:crimson;stroke:green;stroke-width:1"),
                label(t.a, a_label_x, y3, "1em", "middle"),
                label(t.b, x1, b_label_y, "0.5em", "start"),
                label(c, a_label_x, b_label_y, "0em", "end"),
            ))
            .attr("width", 500)
            .attr("height", 200),
            html::div((
                slider(300, t.a, |t, e| t.a = get_value(e)),
                slider(150, t.b, |t, e| t.b = get_value(e)),
            )),
            ml::math(ml::mrow((
                ml::msup((ml::mi(format!("{}", t.a)), ml::mn("2"))),
                ml::mo("+"),
                ml::msup((ml::mi(format!("{}", t.b)), ml::mn("2"))),
                ml::mo("="),
                ml::msup((ml::mi(format!("{c}")), ml::mn("2"))),
            )))
            .attr("style", "width: 100%"),
        ))
    })
    .run(&document_body());
}
