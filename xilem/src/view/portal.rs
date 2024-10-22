// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, ViewId, WidgetView};

/// A view which puts `child` into a scrollable region.
///
/// This corresponds to the Masonry [`Portal`](masonry::widget::Portal) widget.
pub fn portal<Child, State, Action>(child: Child) -> Portal<Child, State, Action>
where
    Child: WidgetView<State, Action>,
{
    Portal {
        child,
        phantom: PhantomData,
    }
}

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
    type Element = Pod<widget::Portal<Child::Widget>>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // The Portal `View` doesn't get any messages directly (yet - scroll events?), so doesn't need to
        // use ctx.with_id.
        let (child, child_state) = self.child.build(ctx);
        let widget_pod = ctx.new_pod(widget::Portal::new_pod(child.inner));
        (widget_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let child_element = widget::Portal::child_mut(&mut element);
        self.child
            .rebuild(&prev.child, view_state, ctx, child_element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let child_element = widget::Portal::child_mut(&mut element);
        self.child.teardown(view_state, ctx, child_element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.child.message(view_state, id_path, message, app_state)
    }
}
