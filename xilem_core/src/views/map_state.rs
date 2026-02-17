// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt::Debug;
use core::marker::PhantomData;

use crate::{MessageCtx, MessageResult, Mut, View, ViewMarker, ViewPathTracker};

/// The View for [`map_state`].
///
/// See its documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct MapState<V, F, ParentState, ChildState, Action, Context> {
    map_state: F,
    child: V,
    phantom: PhantomData<fn(ParentState) -> (ChildState, Action, Context)>,
}

impl<V, F, ParentState, ChildState, Action, Context> Debug
    for MapState<V, F, ParentState, ChildState, Action, Context>
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
/// fn count_view(count: i32) -> impl WidgetView<Edit<i32>> {
///     flex((
///         label(format!("count: {}", count)),
///         button("+", |count| *count += 1),
///         button("-", |count| *count -= 1),
///     ))
/// }
///
/// fn app_logic(state: &mut AppState) -> impl WidgetView<Edit<AppState>> {
///     map_state(count_view(state.count), |state: &mut AppState|  &mut state.count)
/// }
/// ```
pub fn map_state<ParentState, ChildState, Action, Context: ViewPathTracker, V, F>(
    view: V,
    f: F,
) -> MapState<V, F, ParentState, ChildState, Action, Context>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context>,
    F: for<'a> Fn(&'a mut ParentState) -> &'a mut ChildState + 'static,
    MapState<V, F, ParentState, ChildState, Action, Context>: View<ParentState, Action, Context>,
{
    MapState {
        map_state: f,
        child: view,
        phantom: PhantomData,
    }
}

impl<V, F, ParentState, ChildState, Action, Context> ViewMarker
    for MapState<V, F, ParentState, ChildState, Action, Context>
{
}
impl<ParentState, ChildState, Action, Context, V, F> View<ParentState, Action, Context>
    for MapState<V, F, ParentState, ChildState, Action, Context>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context>,
    F: for<'a> Fn(&'a mut ParentState) -> &'a mut ChildState + 'static,
    Action: 'static,
    Context: ViewPathTracker + 'static,
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
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut ParentState,
    ) -> MessageResult<Action> {
        self.child
            .message(view_state, message, element, (&self.map_state)(app_state))
    }
}
