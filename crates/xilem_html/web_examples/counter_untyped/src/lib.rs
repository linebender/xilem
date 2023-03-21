use wasm_bindgen::{prelude::*, JsValue};
use xilem_html::{
    document_body, element as el, on_event, text, App, Event, MessageResult, View, ViewMarker,
};

#[derive(Default)]
struct AppState {
    clicks: i32,
}

impl AppState {
    fn increment(&mut self) -> MessageResult<()> {
        self.clicks += 1;
        MessageResult::Nop
    }
    fn decrement(&mut self) -> MessageResult<()> {
        self.clicks -= 1;
        MessageResult::Nop
    }
    fn reset(&mut self) -> MessageResult<()> {
        self.clicks = 0;
        MessageResult::Nop
    }
}

fn btn<F>(label: &'static str, click_fn: F) -> impl View<AppState> + ViewMarker
where
    F: Fn(&mut AppState, &Event<web_sys::Event, web_sys::HtmlButtonElement>) -> MessageResult<()>,
{
    on_event("click", el("button", text(label)), click_fn)
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    el::<web_sys::HtmlElement, _>(
        "div",
        (
            el::<web_sys::HtmlElement, _>("span", text(format!("clicked {} times", state.clicks))),
            btn("+1 click", |state, _| AppState::increment(state)),
            btn("-1 click", |state, _| AppState::decrement(state)),
            btn("reset clicks", |state, _| AppState::reset(state)),
        ),
    )
}

// Called by our JS entry point to run the example
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    let app = App::new(AppState::default(), app_logic);
    app.run(&document_body());

    Ok(())
}
