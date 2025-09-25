use xilem::{
    EventLoop, InsertNewline, WidgetView, Xilem,
    view::{Axis, button, flex, textbox, virtual_scroll},
    winit::error::EventLoopError,
};

struct State {
    buffer: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            buffer: "".to_string(),
        }
    }
}

fn logic(_state: &mut State) -> impl WidgetView<State> + use<> {
    virtual_scroll(0..1, |state: &mut State, _| {
        flex((
            (button("Clear", |state: &mut State| {
                state.buffer.clear();
            })),
            textbox(
                state.buffer.to_string(),
                |local_state: &mut State, input| local_state.buffer = input,
            )
            .insert_newline(InsertNewline::OnEnter),
        ))
        .direction(Axis::Vertical)
    })
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(State::default(), logic);
    app.run_windowed(EventLoop::with_user_event(), "Textbox Example".into())?;
    Ok(())
}
