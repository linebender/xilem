use wasm_bindgen::{prelude::*, JsValue};
use xilem_html::{
    document_body, elements as el, events as evt, text, App, Event, MessageResult, Text, View,
    ViewExt,
};

#[derive(Default)]
struct AppState {
    clicks: i32,
    class: &'static str,
    text: String,
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
    fn change_class(&mut self) -> MessageResult<()> {
        if self.class == "gray" {
            self.class = "green";
        } else {
            self.class = "gray";
        }
        MessageResult::Nop
    }

    fn change_text(&mut self) -> MessageResult<()> {
        if self.text == "test" {
            self.text = "test2".into();
        } else {
            self.text = "test".into();
        }
        MessageResult::Nop
    }
}

/// You can create functions that generate views.
fn btn<F>(label: &'static str, click_fn: F) -> evt::OnClick<el::Button<Text>, F>
where
    F: Fn(
        &mut AppState,
        &Event<web_sys::MouseEvent, web_sys::HtmlButtonElement>,
    ) -> MessageResult<()>,
{
    el::button(text(label)).on_click(click_fn)
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    el::div((
        el::span(text(format!("clicked {} times", state.clicks))).attr("class", state.class),
        el::br(()),
        btn("+1 click", |state, _| AppState::increment(state)),
        btn("-1 click", |state, _| AppState::decrement(state)),
        btn("reset clicks", |state, _| AppState::reset(state)),
        btn("a different class", |state, _| {
            AppState::change_class(state)
        }),
        btn("change text", |state, _| AppState::change_text(state)),
        el::br(()),
        text(state.text.clone()),
    ))
}

// Called by our JS entry point to run the example
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    let app = App::new(AppState::default(), app_logic);
    app.run(&document_body());

    Ok(())
}
