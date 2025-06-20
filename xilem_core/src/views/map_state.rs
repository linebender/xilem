// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt::Debug;
use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

/// The View for [`map_state`].
///
/// See its documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct MapState<V, F, ParentState, ChildState, Action, Context, Message> {
    map_state: F,
    child: V,
    phantom: PhantomData<fn(ParentState) -> (ChildState, Action, Context, Message)>,
}

impl<V, F, ParentState, ChildState, Action, Context, Message> Debug
    for MapState<V, F, ParentState, ChildState, Action, Context, Message>
where
    V: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapAction")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

/// A view that "extracts" state from a [`View<ParentState,_,_>`] to [`View<ChildState,_,_>`].
/// This allows modularization of views based on their state.
///
/// See also [`lens`](crate::lens), for an alternative with a similar purpose.
///
/// # Examples
///
/// (From the Xilem implementation)
///
/// ```ignore
/// #[derive(Default)]
/// struct AppState {
///     count: i32,
///     other: i32,
/// }
///
/// fn count_view(count: i32) -> impl WidgetView<i32> {
///     flex((
///         label(format!("count: {}", count)),
///         button("+", |count| *count += 1),
///         button("-", |count| *count -= 1),
///     ))
/// }
///
/// fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
///     map_state(count_view(state.count), |state: &mut AppState|  &mut state.count)
/// }
/// ```
pub fn map_state<ParentState, ChildState, Action, Context: ViewPathTracker, Message, V, F>(
    view: V,
    f: F,
) -> MapState<V, F, ParentState, ChildState, Action, Context, Message>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context, Message>,
    F: Fn(&mut ParentState) -> &mut ChildState + 'static,
{
    MapState {
        map_state: f,
        child: view,
        phantom: PhantomData,
    }
}

impl<V, F, ParentState, ChildState, Action, Context, Message> ViewMarker
    for MapState<V, F, ParentState, ChildState, Action, Context, Message>
{
}
impl<ParentState, ChildState, Action, Context, Message, V, F>
    View<ParentState, Action, Context, Message>
    for MapState<V, F, ParentState, ChildState, Action, Context, Message>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context, Message>,
    F: Fn(&mut ParentState) -> &mut ChildState + 'static,
    Action: 'static,
    Context: ViewPathTracker + 'static,
    Message: 'static,
{
    type ViewState = V::ViewState;
    type Element = V::Element;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: &mut ParentState,
    ) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx, (self.map_state)(app_state))
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut ParentState,
    ) {
        self.child.rebuild(
            &prev.child,
            view_state,
            ctx,
            element,
            (self.map_state)(app_state),
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut ParentState,
    ) {
        self.child
            .teardown(view_state, ctx, element, (self.map_state)(app_state));
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut ParentState,
    ) -> MessageResult<Action, Message> {
        self.child
            .message(view_state, id_path, message, (self.map_state)(app_state))
    }
}
