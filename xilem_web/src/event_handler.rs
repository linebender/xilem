use std::{future::Future, marker::PhantomData, rc::Rc};

use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use xilem_core::{MessageResult, ViewPathTracker};

use crate::{context::MessageThunk, DynMessage, Message, ViewCtx};

pub enum EventHandlerMessage<E, Message = DynMessage> {
    Event(E),
    Message(Message),
}

pub trait EventHandler<Event, State, Action, Context: ViewPathTracker, Message = DynMessage>:
    'static
{
    /// State that is used over the lifetime of the retained representation of the event handler.
    ///
    /// This often means routing information for messages to child event handlers or state for async handlers,
    type State;

    /// Init and create the corresponding state.
    fn build(&self, ctx: &mut Context) -> Self::State;

    /// Update handler state based on the difference between `self` and `prev`.
    fn rebuild(&self, prev: &Self, event_handler_state: &mut Self::State, ctx: &mut Context);

    /// Cleanup the handler, when it's being removed from the tree.
    ///
    /// The main use-cases of this method are to:
    /// - Cancel any async tasks
    /// - Clean up any book-keeping set-up in `build` and `rebuild`
    // TODO: Should this take ownership of the `EventHandlerState`
    // We have chosen not to because it makes swapping versions more awkward
    fn teardown(&self, event_handler_state: &mut Self::State, ctx: &mut Context);

    /// Route `message` to `id_path`, if that is still a valid path.
    fn message(
        &self,
        event_handler_state: &mut Self::State,
        id_path: &[xilem_core::ViewId],
        message: EventHandlerMessage<Event, Message>,
        app_state: &mut State,
    ) -> MessageResult<Action, EventHandlerMessage<Event, Message>>;
}

// Because of intersecting trait impls with the blanket impl below, the following impl is unfortunately not possible:
//
// `impl<State, Action, F: Fn(&mut State) -> Action> EventHandler<(), State, Action, ViewCtx> for F {}`
//
// A workaround for this would be to "hardcode" event types, instead of using a blanket impl.
// This is fortunately not a big issue in xilem_web, because there's AFAIK always an event payload (i.e. something different than `()`)

impl<State, Action, Event, Context, Message, F> EventHandler<Event, State, Action, Context, Message>
    for F
where
    Context: ViewPathTracker,
    F: Fn(&mut State, Event) -> Action + 'static,
{
    type State = ();

    fn build(&self, _ctx: &mut Context) -> Self::State {}

    fn rebuild(&self, _prev: &Self, _event_handler_state: &mut Self::State, _ctx: &mut Context) {}

    fn teardown(&self, _event_handler_state: &mut Self::State, _ctx: &mut Context) {}

    fn message(
        &self,
        _event_handler_state: &mut Self::State,
        id_path: &[xilem_core::ViewId],
        message: EventHandlerMessage<Event, Message>,
        app_state: &mut State,
    ) -> MessageResult<Action, EventHandlerMessage<Event, Message>> {
        debug_assert!(id_path.is_empty());
        match message {
            EventHandlerMessage::Event(event) => MessageResult::Action(self(app_state, event)),
            EventHandlerMessage::Message(_) => unreachable!(),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeferEventHandler<T, A, FO, F, FF, CF> {
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> (T, A, FO, F)>,
    future_fn: FF,
    callback_fn: CF,
}

impl<State, Action, Event, FO, F, FF, CF> EventHandler<Event, State, Action, ViewCtx>
    for DeferEventHandler<State, Action, FO, F, FF, CF>
where
    State: 'static,
    Action: 'static,
    Event: 'static,
    FO: Message,
    F: Future<Output = FO> + 'static,
    FF: Fn(&mut State, Event) -> F + 'static,
    CF: Fn(&mut State, FO) + 'static,
{
    type State = Rc<MessageThunk>;

    fn build(&self, ctx: &mut ViewCtx) -> Self::State {
        Rc::new(ctx.message_thunk())
    }

    fn rebuild(&self, _prev: &Self, _event_handler_state: &mut Self::State, _ctx: &mut ViewCtx) {}

    fn teardown(&self, _event_handler_state: &mut Self::State, _ctx: &mut ViewCtx) {}

    fn message(
        &self,
        message_thunk: &mut Self::State,
        id_path: &[xilem_core::ViewId],
        message: EventHandlerMessage<Event>,
        app_state: &mut State,
    ) -> MessageResult<Action, EventHandlerMessage<Event>> {
        debug_assert!(id_path.is_empty());
        match message {
            EventHandlerMessage::Event(event) => {
                let future = (self.future_fn)(app_state, event);
                let thunk = Rc::clone(message_thunk);
                // TODO currently, multiple events could trigger this, while the (old) future is still not resolved
                // This may be intended, but can also lead to surprising behavior.
                // We could add an atomic boolean, to avoid this, i.e. either block a new future,
                // or even queue it after the first future being resolved, there may also be other possible desired behaviors
                // This could also be made configurable, e.g. via the builder pattern, like this:
                // ```
                // defer(...)
                //     .block() // block new events triggering that future
                //     .once() // allow this event to trigger the future only once.
                //     .queue() // queue additional triggered futures
                // ```
                spawn_local(async move { thunk.push_message(future.await) });
                MessageResult::RequestRebuild
            }
            EventHandlerMessage::Message(output) => {
                (self.callback_fn)(app_state, *output.downcast::<FO>().unwrap_throw());
                MessageResult::RequestRebuild
            }
        }
    }
}

pub fn defer<State, Action, Event, FO, F, FF, CF>(
    future_fn: FF,
    callback_fn: CF,
) -> DeferEventHandler<State, Action, FO, F, FF, CF>
where
    State: 'static,
    Action: 'static,
    Event: 'static,
    FO: Message,
    F: Future<Output = FO> + 'static,
    FF: Fn(&mut State, Event) -> F + 'static,
    CF: Fn(&mut State, FO) + 'static,
{
    DeferEventHandler {
        phantom: PhantomData,
        future_fn,
        callback_fn,
    }
}
