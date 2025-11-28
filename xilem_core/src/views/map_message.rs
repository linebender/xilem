// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt::Debug;
use core::marker::PhantomData;

use crate::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker, ViewPathTracker};

/// View type for [`map_message`] and [`map_action`]. Most users will want to use `map_action` (the latter).
///
/// This view maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`], whilst allowing the kind of [`MessageResult`] to be changed.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct MapMessage<
    V,
    State,
    ParentAction,
    ChildAction,
    Context,
    // This default only exists for documentation purposes.
    F = fn(Arg<'_, State>, ChildAction) -> ParentAction,
> {
    map_fn: F,
    child: V,
    phantom: PhantomData<fn() -> (State, ParentAction, ChildAction, Context)>,
}

impl<V, State, ParentAction, ChildAction, Context, F> Debug
    for MapMessage<V, State, ParentAction, ChildAction, Context, F>
where
    V: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapAction")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
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
/// fn count_view<T>(count: i32) -> impl WidgetView<Edit<T>, CountMessage> {
///     flex((
///         label(format!("count: {}", count)),
///         button("+", |_| CountMessage::Increment),
///         button("-", |_| CountMessage::Decrement),
///     ))
/// }
///
/// fn app_logic(count: &mut i32) -> impl WidgetView<Edit<i32>> {
///     map_action(count_view(*count), |count, message| match message {
///         CountMessage::Increment => *count += 1,
///         CountMessage::Decrement => *count -= 1,
///     })
/// }
/// ```
pub fn map_action<State, ParentAction, ChildAction, Context: ViewPathTracker, V, F>(
    view: V,
    map_fn: F,
) -> MapMessage<
    V,
    State,
    ParentAction,
    ChildAction,
    Context,
    impl Fn(Arg<'_, State>, MessageResult<ChildAction>) -> MessageResult<ParentAction> + 'static,
>
where
    State: ViewArgument,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context>,
    F: Fn(Arg<'_, State>, ChildAction) -> ParentAction + 'static,
{
    MapMessage {
        map_fn: move |app_state: Arg<'_, State>, result: MessageResult<ChildAction>| {
            result.map(|action| map_fn(app_state, action))
        },
        child: view,
        phantom: PhantomData,
    }
}

/// A view which maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`], whilst allowing the kind of [`MessageResult`] to be changed.
///
/// This is the more general form of [`map_action`].
/// In most cases, you probably want to use that function.
pub fn map_message<State, ParentAction, ChildAction, Context: ViewPathTracker, V, F>(
    view: V,
    map_fn: F,
) -> MapMessage<V, State, ParentAction, ChildAction, Context, F>
where
    State: ViewArgument,
    ParentAction: 'static,
    ChildAction: 'static,
    V: View<State, ChildAction, Context>,
    F: Fn(Arg<'_, State>, MessageResult<ChildAction>) -> MessageResult<ParentAction> + 'static,
{
    MapMessage {
        map_fn,
        child: view,
        phantom: PhantomData,
    }
}

impl<V, State, ParentAction, ChildAction, F, Context> ViewMarker
    for MapMessage<V, State, ParentAction, ChildAction, Context, F>
{
}
impl<V, State, ParentAction, ChildAction, Context, F> View<State, ParentAction, Context>
    for MapMessage<V, State, ParentAction, ChildAction, Context, F>
where
    V: View<State, ChildAction, Context>,
    State: ViewArgument,
    ParentAction: 'static,
    ChildAction: 'static,
    F: Fn(Arg<'_, State>, MessageResult<ChildAction>) -> MessageResult<ParentAction> + 'static,
    Context: ViewPathTracker + 'static,
{
    type ViewState = V::ViewState;
    type Element = V::Element;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx, app_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        self.child
            .rebuild(&prev.child, view_state, ctx, element, app_state);
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
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        mut app_state: Arg<'_, State>,
    ) -> MessageResult<ParentAction> {
        let child_result = self.child.message(
            view_state,
            message,
            element,
            State::reborrow_mut(&mut app_state),
        );
        (self.map_fn)(app_state, child_result)
    }
}
