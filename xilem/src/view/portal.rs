// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widgets;

use crate::core::{MessageContext, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, WidgetView};

/// A view which puts `child` into a scrollable region.
///
/// This corresponds to the Masonry [`Portal`](masonry::widgets::Portal) widget.
pub fn portal<Child, State, Action>(child: Child) -> Portal<Child, State, Action>
where
    Child: WidgetView<State, Action>,
{
    Portal {
        child,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`portal`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Portal<V, State, Action> {
    child: V,
    phantom: PhantomData<(State, Action)>,
}

impl<V, State, Action> ViewMarker for Portal<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for Portal<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<widgets::Portal<Child::Widget>>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // The Portal `View` doesn't get any messages directly (yet - scroll events?), so doesn't need to
        // use ctx.with_id.
        let (child, child_state) = self.child.build(ctx, app_state);
        let widget_pod = ctx.create_pod(widgets::Portal::new(child.new_widget));
        (widget_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let child_element = widgets::Portal::child_mut(&mut element);
        self.child
            .rebuild(&prev.child, view_state, ctx, child_element, app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let child_element = widgets::Portal::child_mut(&mut element);
        self.child
            .teardown(view_state, ctx, child_element, app_state);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let child_element = widgets::Portal::child_mut(&mut element);
        self.child
            .message(view_state, message, child_element, app_state)
    }
}
