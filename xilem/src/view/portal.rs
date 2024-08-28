// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget;
use xilem_core::ViewMarker;

use crate::{
    core::{Mut, View, ViewId},
    Pod, ViewCtx, WidgetView,
};

/// A scrollable widget portal
pub fn portal<State, Action, V>(inner: V) -> Portal<V, State, Action>
where
    V: WidgetView<State, Action>,
{
    Portal {
        inner,
        phantom: PhantomData,
    }
}

pub struct Portal<V, State, Action = ()> {
    inner: V,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> Portal<V, State, Action> {}

impl<V, State, Action> ViewMarker for Portal<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx> for Portal<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widget::Portal>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.inner.build(ctx);
        let widget = widget::Portal::new_pod(child.inner.boxed()).content_must_fill(true);
        (Pod::new(widget), child_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            let mut child = element.child_mut();
            self.inner
                .rebuild(&prev.inner, view_state, ctx, child.downcast());
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = element.child_mut();
        self.inner.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.inner.message(view_state, id_path, message, app_state)
    }
}
