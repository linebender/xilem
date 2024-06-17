use core::marker::PhantomData;

use crate::{DynMessage, Mut, View, ViewId, ViewPathTracker};

/// A view that maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`] while providing mutable access to `State` in the map function.
///
/// This is very similar to the Elm architecture, where the parent view can update state based on the action message from the child view.
pub struct MapAction<
    State,
    ParentAction,
    ChildAction,
    V,
    F = fn(&mut State, ChildAction) -> ParentAction,
> {
    map_fn: F,
    child: V,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<fn() -> (State, ParentAction, ChildAction)>,
}

/// A view that maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`] while providing mutable access to `State` in the map function.
///
/// This is very similar to the Elm architecture, where the parent view can update state based on the action message from the child view.
///
/// # Examples
///
/// (From the Xilem implementation)
///
/// ```ignore
/// enum CountMessage {
///     Increment,
///     Decrement,
/// }
///
/// fn count_view<T>(count: i32) -> impl WidgetView<T, CountMessage> {
///     flex((
///         label(format!("count: {}", count)),
///         button("+", |_| CountMessage::Increment),
///         button("-", |_| CountMessage::Decrement),
///     ))
/// }
///
/// fn main() -> Result<(), EventLoopError> {
///     Xilem::new(0, |count| {
///         map_action(count_view(*count), |count, message| match message {
///             CountMessage::Increment => *count += 1,
///             CountMessage::Decrement => *count -= 1,
///         })
///     })
///     .run_windowed(EventLoop::with_user_event(), "Map action example".into())?;
///     Ok(())
/// }
/// ```
pub fn map_action<State, ParentAction, ChildAction, Context: ViewPathTracker, V, F>(
    view: V,
    map_fn: F,
) -> MapAction<State, ParentAction, ChildAction, V, F>
where
    State: 'static,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context>,
    F: Fn(&mut State, ChildAction) -> ParentAction + 'static,
{
    MapAction {
        map_fn,
        child: view,
        phantom: PhantomData,
    }
}

impl<State, ParentAction, ChildAction, Context: ViewPathTracker, V, F>
    View<State, ParentAction, Context> for MapAction<State, ParentAction, ChildAction, V, F>
where
    State: 'static,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context>,
    F: Fn(&mut State, ChildAction) -> ParentAction + 'static,
{
    type ViewState = V::ViewState;
    type Element = V::Element;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        self.child.rebuild(&prev.child, view_state, ctx, element)
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<ParentAction> {
        self.child
            .message(view_state, id_path, message, app_state)
            .map(|action| (self.map_fn)(app_state, action))
    }
}
