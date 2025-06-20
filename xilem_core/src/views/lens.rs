// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::any::type_name;
use core::fmt::Debug;
use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

/// The View for [`lens`].
///
/// See its documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Lens<CF, V, F, ParentState, ChildState, Action, Context, Message> {
    access_state: F,
    child_component: CF,
    phantom: PhantomData<fn(ParentState) -> (ChildState, Action, Context, Message, V)>,
}

impl<CF, V, F, ParentState, ChildState, Action, Context, Message> Debug
    for Lens<CF, V, F, ParentState, ChildState, Action, Context, Message>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Lens")
            .field("from", &type_name::<ParentState>())
            .field("to", &type_name::<ChildState>())
            .finish_non_exhaustive()
    }
}

/// An adapter which allows using a component which only uses one field of the current state.
///
/// In Xilem, many components are functions of the form `fn my_component(&mut SomeState) -> impl WidgetView<SomeState>`.
/// For example, a date picker might be of the form `fn date_picker(&mut Date) -> impl WidgetView<Date>`.
///
/// The `lens` View allows using these components in a higher-level component, where the higher level state has
/// a field of the inner component's state type.
/// For example, a flight finder app might have a `Date` field for the currently selected date.
///
/// The parameters of this view are:
/// - `component`: The child component the lens is being created for.
/// - `map`: A function from the higher-level state type to `component`'s state type
///
/// That view can be used if the child doesn't follow the expected component signature.
///
/// It's a more specialized/simpler alternative to [`map_state`](crate::map_state)
///
/// # Examples
///
/// In code, the date picker example might look like:
///
/// ```
/// # use xilem_core::docs::{DocsView as WidgetView, State as Date, State as Flight, some_component};
/// use xilem_core::lens;
///
/// fn date_picker(date: &mut Date) -> impl WidgetView<Date> + use<> {
/// # some_component(date)
/// // ...
/// }
///
/// fn app_logic(state: &mut FlightPlanner) -> impl WidgetView<FlightPlanner> {
///     lens(date_picker, |state: &mut FlightPlanner| &mut state.date)
/// }
///
/// struct FlightPlanner {
///     date: Date,
///     available_flights: Vec<Flight>,
/// }
/// ```
pub fn lens<OuterState, Action, Context, Message, InnerState, StateF, InnerView, Component>(
    component: Component,
    // This parameter ordering does run into https://github.com/rust-lang/rustfmt/issues/3605
    // Our general advice is to make sure that the lens arguments are short enough...
    access_state: StateF,
) -> Lens<Component, InnerView, StateF, OuterState, InnerState, Action, Context, Message>
where
    StateF: Fn(&mut OuterState) -> &mut InnerState + Send + Sync + 'static,
    Component: Fn(&mut InnerState) -> InnerView,
    InnerView: View<InnerState, Action, Context, Message>,
    Context: ViewPathTracker,
{
    // TODO: allow either of these? But we can't easily detect whether the functions have changed. Do a similar pattern as within worker_raw?
    const {
        assert!(
            size_of::<Component>() == 0,
            "The component function needs to be a non capturing closure"
        );
        assert!(
            size_of::<StateF>() == 0,
            "The child state access function needs to be a non capturing closure"
        );
    }
    Lens {
        child_component: component,
        access_state,
        phantom: PhantomData,
    }
}

impl<Component, V, StateF, ParentState, ChildState, Action, Context, Message> ViewMarker
    for Lens<Component, V, StateF, ParentState, ChildState, Action, Context, Message>
{
}
impl<Component, ParentState, ChildState, Action, Context, Message, V, StateF>
    View<ParentState, Action, Context, Message>
    for Lens<Component, V, StateF, ParentState, ChildState, Action, Context, Message>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState, Action, Context, Message>,
    Component: Fn(&mut ChildState) -> V + 'static,
    StateF: Fn(&mut ParentState) -> &mut ChildState + 'static,
    Action: 'static,
    Context: ViewPathTracker + 'static,
    Message: 'static,
{
    type ViewState = (V, V::ViewState);
    type Element = V::Element;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: &mut ParentState,
    ) -> (Self::Element, Self::ViewState) {
        let child_state = (self.access_state)(app_state);
        let child = (self.child_component)(child_state);
        let (element, child_state) = child.build(ctx, (self.access_state)(app_state));
        (element, (child, child_state))
    }

    fn rebuild(
        &self,
        _prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut ParentState,
    ) {
        let child_state = (self.access_state)(app_state);
        let child = (self.child_component)(child_state);
        child.rebuild(&view_state.0, &mut view_state.1, ctx, element, child_state);
        view_state.0 = child;
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut ParentState,
    ) {
        let child_state = (self.access_state)(app_state);
        let child = (self.child_component)(child_state);
        child.teardown(&mut view_state.1, ctx, element, child_state);
    }

    fn message(
        &self,
        (child, child_view_state): &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut ParentState,
    ) -> MessageResult<Action, Message> {
        child.message(
            child_view_state,
            id_path,
            message,
            (self.access_state)(app_state),
        )
    }
}
