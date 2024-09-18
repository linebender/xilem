// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

/// The View for [`map_state`] and [`lens`].
///
/// See their documentation for more context.
pub struct MapState<V, F, ParentState, ChildState, Action, Context, Message> {
    map_state: F,
    child: V,
    phantom: PhantomData<fn(ParentState) -> (ChildState, Action, Context, Message)>,
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

/// An adapter which allows using a component which only uses one field of the current state.
///
/// In Xilem, many components are functions of the form `fn my_component(&mut SomeState) -> impl WidgetView<SomeState>`.
/// For example, a date picker might be of the form `fn date_picker(&mut Date) -> impl WidgetView<Date>`.
/// The `lens` View allows using these components in a higher-level component, where the higher level state has
/// a field of the inner component's state type.
/// For example, a flight finder app might have a `Date` field for the currently selected date.
///
/// The parameters of this view are:
/// - `component`: The child component the lens is being created for.
/// - `state`: The current outer view's state
/// - `map`: A function from the higher-level state type to `component`'s state type
///
/// This is a wrapper around [`map_state`].
/// That view can be used if the child doesn't follow the expected component signature.
///
/// # Examples
///
/// In code, the date picker example might look like:
///
/// ```
/// # use xilem_core::docs::{DocsView as WidgetView, State as Date, State as Flight, some_component as date_picker};
/// use xilem_core::lens;
///
/// fn app_logic(state: &mut FlightPlanner) -> impl WidgetView<FlightPlanner> {
///     lens(date_picker, state, |state| &mut state.date)
/// }
///
/// struct FlightPlanner {
///     date: Date,
///     available_flights: Vec<Flight>,
/// }
/// ```
pub fn lens<OuterState, Action, Context, Message, InnerState, StateF, InnerView, Component>(
    component: Component,
    state: &mut OuterState,
    // This parameter ordering does run into https://github.com/rust-lang/rustfmt/issues/3605
    // Our general advice is to make sure that the lens arguments are short enough...
    map: StateF,
) -> MapState<InnerView, StateF, OuterState, InnerState, Action, Context, Message>
where
    StateF: Fn(&mut OuterState) -> &mut InnerState + Send + Sync + 'static,
    Component: FnOnce(&mut InnerState) -> InnerView,
    InnerView: View<InnerState, Action, Context, Message>,
    Context: ViewPathTracker,
{
    let mapped = map(state);
    let view = component(mapped);
    MapState {
        child: view,
        map_state: map,
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
            .message(view_state, id_path, message, (self.map_state)(app_state))
    }
}
