# `xilem_web`

This is a prototype of a potential implementation of the Xilem architecture (implementing `xilem_core`) using DOM elements as Xilem elements (unfortunately the two concepts have the same name).

The easiest way to run it is to use [Trunk] (see the examples in the `web_examples/` directory).
Run `trunk serve`, then navigate the browser to the link provided (usually <http://localhost:8080>).

## Example

```rust
use xilem_web::{
    document_body,
    elements::html::{button, div, p},
    interfaces::{Element, HtmlDivElement},
    App,
};

fn app_logic(clicks: &mut u32) -> impl HtmlDivElement<u32> {
    div((
        button(format!("clicked {clicks} times")).on_click(|clicks: &mut u32, _event| *clicks += 1),
        (*clicks >= 5).then_some(p("Huzzah, clicked at least 5 times")),
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    let clicks = 0;
    App::new(document_body(), clicks, app_logic).run();
}
```

[Trunk]: https://trunkrs.dev/
