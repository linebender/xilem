// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::type_name;
use std::marker::PhantomData;
use vello::kurbo::Size;
use xilem_core::{Arg, MessageResult, ViewArgument, ViewId, ViewPathTracker};

use masonry::widgets::{self, LayoutChanged};

use crate::core::{MessageContext, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A view which calls the `on_resize` callback whenever the size of its child changes.
///
/// `on_resize` is a function which takes the app's state and the new [`Size`] of the view.
///
/// This can be a useful primitive for making size-adaptive designs, such as
/// scaling up a game board in response more space being available, or switching
/// to use fewer columns when there is not space to fit multiple columns.
/// This can be safely used to dynamically access the size of a window
/// or tab in a [`split`](crate::view::split::split).
/// You must make sure that the child takes up all the available space.
/// This can be most easily achieved by making the child be
/// an [expanded](crate::view::SizedBox::expand) `sized_box`.
///
/// See the documentation on the underlying [`ResizeObserver`](widgets::ResizeObserver) for more information.
pub fn resize_observer<State, Action, V, F>(
    on_resize: F,
    inner: V,
) -> ResizeObserver<V, F, State, Action>
where
    V: WidgetView<State, Action>,
    F: Fn(Arg<'_, State>, Size) -> Action,
    State: ViewArgument,
    ResizeObserver<V, F, State, Action>: WidgetView<State, Action>,
{
    ResizeObserver {
        inner,
        on_resize,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`resize_observer`].
///
/// See `resize_observer` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ResizeObserver<V, F, State, Action = ()> {
    inner: V,
    on_resize: F,
    phantom: PhantomData<fn() -> (State, Action)>,
}

const RESIZE_OBSERVER_CONTENT_VIEW_ID: ViewId = ViewId::new(0);

impl<V, F, State, Action> ViewMarker for ResizeObserver<V, F, State, Action> {}
impl<V, F, State, Action> View<State, Action, ViewCtx> for ResizeObserver<V, F, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    F: 'static,
    V: WidgetView<State, Action>,
    F: Fn(Arg<'_, State>, Size) -> Action,
{
    type Element = Pod<widgets::ResizeObserver>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(RESIZE_OBSERVER_CONTENT_VIEW_ID, |ctx| {
            self.inner.build(ctx, app_state)
        });
        (
            ctx.with_action_widget(|ctx| {
                ctx.create_pod(widgets::ResizeObserver::new(child.new_widget))
            }),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        ctx.with_id(RESIZE_OBSERVER_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.inner,
                &prev.inner,
                view_state,
                ctx,
                widgets::ResizeObserver::child_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(RESIZE_OBSERVER_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.inner,
                view_state,
                ctx,
                widgets::ResizeObserver::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        mut app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(RESIZE_OBSERVER_CONTENT_VIEW_ID) => self.inner.message(
                view_state,
                message,
                widgets::ResizeObserver::child_mut(&mut element).downcast(),
                State::reborrow_mut(&mut app_state),
            ),
            None => match message.take_message::<LayoutChanged>() {
                Some(_) => MessageResult::Action((self.on_resize)(app_state, element.ctx.size())),
                None => {
                    // TODO: Panic?
                    tracing::error!(
                        "Wrong message type in ResizeObserver::message: {message:?} expected {}",
                        type_name::<LayoutChanged>()
                    );
                    MessageResult::Stale
                }
            },
            _ => {
                tracing::warn!("Got unexpected id path in `ResizeObserver::message`.");
                MessageResult::Stale
            }
        }
    }
}
