// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::type_name;
use std::marker::PhantomData;

use masonry::kurbo::Size;
use masonry::properties::Dimensions;
use masonry::widgets::{self, LayoutChanged};

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
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
///
/// See the documentation on the underlying [`ResizeObserver`](widgets::ResizeObserver) for more information.
///
/// # Example
///
/// To create a responsive version of a page, you can use `resize_observer`.
///
/// ```rust,no_run
/// # use xilem_masonry as xilem;
/// # use xilem::{WidgetView, view::{resize_observer, flex}, masonry::kurbo::{Size, Axis}};
///
/// struct State {
///     available_size: Size,
/// }
///
/// # fn my_component(state: &mut State) -> impl WidgetView<State> {
/// resize_observer(
///     |state: &mut State, size: Size| state.available_size = size,
///     flex(
///         // Horizontal on wide screens,
///         if state.available_size.width > 1000. {
///             Axis::Horizontal
///         } else {
///             Axis::Vertical
///         },
///         (
///             // ...
///         ),
///     )
/// )
/// # }
/// ```
pub fn resize_observer<State, Action, V, F>(
    on_resize: F,
    inner: V,
) -> ResizeObserver<V, F, State, Action>
where
    V: WidgetView<State, Action>,
    F: Fn(&mut State, Size) -> Action,
    State: 'static,
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

// Use a distinctive number here, to be able to catch bugs.
/// This is a randomly generated 32 bit number - 2850384319 in decimal.
const RESIZE_OBSERVER_CONTENT_VIEW_ID: ViewId = ViewId::new(0xa9e569bf);

impl<V, F, State, Action> ViewMarker for ResizeObserver<V, F, State, Action> {}
impl<V, F, State, Action> View<State, Action, ViewCtx> for ResizeObserver<V, F, State, Action>
where
    State: 'static,
    Action: 'static,
    F: 'static,
    V: WidgetView<State, Action>,
    F: Fn(&mut State, Size) -> Action,
{
    type Element = Pod<widgets::ResizeObserver>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(RESIZE_OBSERVER_CONTENT_VIEW_ID, |ctx| {
            self.inner.build(ctx, app_state)
        });
        (
            ctx.with_action_widget(|_| {
                let widget = widgets::ResizeObserver::new(child.new_widget);
                Pod::new_with_props(widget, Dimensions::MAX)
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
        app_state: &mut State,
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
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        mut app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(RESIZE_OBSERVER_CONTENT_VIEW_ID) => self.inner.message(
                view_state,
                message,
                widgets::ResizeObserver::child_mut(&mut element).downcast(),
                &mut app_state,
            ),
            None => match message.take_message::<LayoutChanged>() {
                Some(_) => MessageResult::Action((self.on_resize)(
                    app_state,
                    element.ctx.content_box_size(),
                )),
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
                tracing::warn!(
                    ?message,
                    "Got unexpected id path in `ResizeObserver::message`."
                );
                MessageResult::Stale
            }
        }
    }
}
