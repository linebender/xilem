use xilem_html::{document_body, element, on_event, App, Event, View, ViewMarker};

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

fn btn<F>(label: &'static str, click_fn: F) -> impl View<AppState> + ViewMarker
where
    F: Fn(&mut AppState, &Event<web_sys::Event, web_sys::HtmlButtonElement>),
{
    on_event(
        "click",
        element("button", label),
        move |state: &mut AppState, evt: &Event<_, _>| {
            click_fn(state, evt);
        },
    )
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    element::<web_sys::HtmlElement, _>(
        "div",
        (
            element::<web_sys::HtmlElement, _>("span", format!("clicked {} times", state.clicks)),
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
