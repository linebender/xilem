use xilem_html::{
    document_body,
    elements::custom_element,
    interfaces::{EventTarget, HtmlElement},
    App, View,
};

#[derive(Default)]
struct AppState {
    clicks: i32,
}

impl AppState {
    fn increment(&mut self) {
        self.clicks += 1;
    }
    fn decrement(&mut self) {
        self.clicks -= 1;
    }
    fn reset(&mut self) {
        self.clicks = 0;
    }
}

fn btn(
    label: &'static str,
    click_fn: impl Fn(&mut AppState, web_sys::Event),
) -> impl HtmlElement<AppState> {
    custom_element("button", label).on("click", move |state: &mut AppState, evt| {
        click_fn(state, evt);
    })
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    custom_element(
        "div",
        (
            custom_element("span", format!("clicked {} times", state.clicks)),
            btn("+1 click", |state, _| AppState::increment(state)),
            btn("-1 click", |state, _| AppState::decrement(state)),
            btn("reset clicks", |state, _| AppState::reset(state)),
        ),
    )
}

pub fn main() {
    console_error_panic_hook::set_once();
    let app = App::new(AppState::default(), app_logic);
    app.run(&document_body());
}
