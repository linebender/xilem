// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{Mut, View, ViewId, ViewMarker, ViewPathTracker};

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
/// fn app_logic(count: &mut i32) -> impl WidgetView<i32> {
///     map_action(count_view(*count), |count, message| match message {
///         CountMessage::Increment => *count += 1,
///         CountMessage::Decrement => *count -= 1,
///     })
/// }
/// ```
pub fn map_action<State, ParentAction, ChildAction, Context: ViewPathTracker, Message, V, F>(
    view: V,
    map_fn: F,
) -> MapAction<State, ParentAction, ChildAction, V, F>
where
    State: 'static,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context, Message>,
    F: Fn(&mut State, ChildAction) -> ParentAction + 'static,
{
    MapAction {
        map_fn,
        child: view,
        phantom: PhantomData,
    }
}

impl<State, ParentAction, ChildAction, V, F> ViewMarker
    for MapAction<State, ParentAction, ChildAction, V, F>
{
}
impl<State, ParentAction, ChildAction, Context: ViewPathTracker, Message, V, F>
    View<State, ParentAction, Context, Message>
    for MapAction<State, ParentAction, ChildAction, V, F>
where
    State: 'static,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context, Message>,
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
        message: Message,
        app_state: &mut State,
    ) -> crate::MessageResult<ParentAction, Message> {
        self.child
            .message(view_state, id_path, message, app_state)
            .map(|action| (self.map_fn)(app_state, action))
    }
}
