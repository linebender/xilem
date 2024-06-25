// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewPathTracker};

/// A view that "extracts" state from a [`View<ParentState,_,_>`] to [`View<ChildState,_,_>`].
/// This allows modularization of views based on their state.
pub struct MapState<ParentState, ChildState, V, F = fn(&mut ParentState) -> &mut ChildState> {
    f: F,
    child: V,
    phantom: PhantomData<fn() -> (ParentState, ChildState)>,
}

/// A view that "extracts" state from a [`View<ParentState,_,_>`] to [`View<ChildState,_,_>`].
/// This allows modularization of views based on their state.
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
) -> MapState<ParentState, ChildState, V, F>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context, Message>,
    F: Fn(&mut ParentState) -> &mut ChildState + 'static,
{
    MapState {
        f,
        child: view,
        phantom: PhantomData,
    }
}

impl<ParentState, ChildState, Action, Context: ViewPathTracker, Message, V, F>
    View<ParentState, Action, Context, Message> for MapState<ParentState, ChildState, V, F>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context, Message>,
    F: Fn(&mut ParentState) -> &mut ChildState + 'static,
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
        app_state: &mut ParentState,
    ) -> MessageResult<Action, Message> {
        self.child
            .message(view_state, id_path, message, (self.f)(app_state))
    }
}
