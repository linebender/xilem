//! A mastodon client written in Xilem.
//!
//! Features:
//!
//! - None

use xilem::{
    EventLoopBuilder, WidgetView, WindowOptions, Xilem, view::label, winit::error::EventLoopError,
};

struct Placehero {}

fn app_logic(_app_state: &mut Placehero) -> impl WidgetView<Placehero> + use<> {
    label("Nothing to see here")
}

/// Execute the app in the given winit event loop.
pub fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let app_state = Placehero {};

    Xilem::new_simple(
        app_state,
        app_logic,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}
