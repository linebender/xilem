// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::rc::Rc;
use alloc::sync::Arc;
use core::ops::Deref;

use crate::message::MessageResult;
use crate::{Arg, MessageCtx, Mut, View, ViewArgument, ViewMarker, ViewPathTracker};

#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
#[derive(Debug)]
pub struct RcState<ViewState> {
    pub(crate) view_state: ViewState,
    /// This is a flag that is set, when an inner view signifies that it requires a rebuild (via [`MessageResult::RequestRebuild`]).
    /// This can happen, e.g. when an inner view wasn't changed by the app-developer directly (i.e. it points to the same view),
    /// but e.g. through some kind of async action.
    /// An example would be an async virtualized list, which fetches new entries, and requires a rebuild for the new entries.
    pub(crate) dirty: bool,
}

impl<V: ?Sized> ViewMarker for Rc<V> {}
/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context, V> View<State, Action, Context> for Rc<V>
where
    State: ViewArgument,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ?Sized,
{
    type Element = V::Element;
    type ViewState = RcState<V::ViewState>;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (element, view_state) = self.deref().build(ctx, app_state);
        (
            element,
            RcState {
                view_state,
                dirty: false,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        #![expect(clippy::use_self, reason = "`Rc::ptr_eq` is the canonical form")]
        if core::mem::take(&mut view_state.dirty) || !Rc::ptr_eq(self, prev) {
            self.deref()
                .rebuild(prev, &mut view_state.view_state, ctx, element, app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.deref()
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let message_result =
            self.deref()
                .message(&mut view_state.view_state, message, element, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}

impl<V: ?Sized> ViewMarker for Arc<V> {}
/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context, V> View<State, Action, Context> for Arc<V>
where
    State: ViewArgument,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ?Sized,
{
    type Element = V::Element;
    type ViewState = RcState<V::ViewState>;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (element, view_state) = self.deref().build(ctx, app_state);
        (
            element,
            RcState {
                view_state,
                dirty: false,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        #![expect(clippy::use_self, reason = "`Arc::ptr_eq` is the canonical form")]
        if core::mem::take(&mut view_state.dirty) || !Arc::ptr_eq(self, prev) {
            self.deref()
                .rebuild(prev, &mut view_state.view_state, ctx, element, app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.deref()
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let message_result =
            self.deref()
                .message(&mut view_state.view_state, message, element, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}
