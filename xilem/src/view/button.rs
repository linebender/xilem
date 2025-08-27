// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::type_name;

pub use masonry::core::PointerButton;
use masonry::widgets::{self, ButtonPress};

use crate::core::{MessageContext, Mut, View, ViewMarker, ViewPathTracker};
use crate::view::Label;
use crate::{MessageResult, Pod, ViewCtx, ViewId, WidgetView};

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
///
/// # Examples
/// To use button provide it with a button text and a closure.
/// ```
/// use xilem::view::{button, label};
/// # use xilem::WidgetView;
///
/// struct State {
///     int: i32,
/// }
///
/// impl State {
///     fn increase(&mut self) {
///         self.int += 1;
///     }
/// }
///
/// # fn view() -> impl WidgetView<State> {
/// button("Button", |state: &mut State| {
///      state.increase();
/// })
/// # }
/// ```
///
/// Create a `button` with a custom `label`.
///
/// ```
/// use xilem::{view::{button, label}, FontWeight};
/// # use xilem::WidgetView;
///
/// struct State {
///     int: i32,
/// }
///
/// impl State {
///     fn increase(&mut self) {
///         self.int += 1;
///     }
/// }
///
/// # fn view() -> impl WidgetView<State> {
/// let label = label("Button").weight(FontWeight::BOLD);
///
/// button(label, |state: &mut State| {
///     state.increase();
/// })
/// # }
/// ```
pub fn button<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button<
    impl for<'a> Fn(&'a mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
    Label,
> {
    any_button(label.into(), callback)
}

/// See [`button`], the only difference is, that it allows arbitrary widgets as content.
///
/// `child` should be a non-interactive widget, like a [`label`](crate::view::label::label)
pub fn any_button<State, Action, V: WidgetView<State, Action>>(
    child: V,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button<
    impl for<'a> Fn(&'a mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
    V,
> {
    Button {
        child,
        callback: move |state: &mut State, button| match button {
            None | Some(PointerButton::Primary) => MessageResult::Action(callback(state)),
            _ => MessageResult::Nop,
        },
        disabled: false,
    }
}

/// A button which calls `callback` when pressed.
pub fn button_any_pointer<State, Action, V: WidgetView<State, Action>>(
    child: V,
    callback: impl Fn(&mut State, Option<PointerButton>) -> Action + Send + 'static,
) -> Button<
    impl for<'a> Fn(&'a mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
    V,
> {
    Button {
        child,
        callback: move |state: &mut State, button| MessageResult::Action(callback(state, button)),
        disabled: false,
    }
}

/// The [`View`] created by [`button`] from a `label` and a callback.
///
/// See `button` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button<F, V> {
    child: V,
    callback: F,
    disabled: bool,
}

impl<F, V> Button<F, V> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

const BUTTON_CONTENT_VIEW_ID: ViewId = ViewId::new(0);

impl<F, V> ViewMarker for Button<F, V> {}
impl<F, V, State, Action> View<State, Action, ViewCtx> for Button<F, V>
where
    V: WidgetView<State, Action>,
    F: Fn(&mut State, Option<PointerButton>) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widgets::Button>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(BUTTON_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.child, ctx, app_state)
        });
        (
            ctx.with_action_widget(|ctx| {
                let mut pod = ctx.create_pod(widgets::Button::new(child.new_widget));
                pod.new_widget.options.disabled = self.disabled;
                pod
            }),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        ctx.with_id(BUTTON_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.child,
                &prev.child,
                state,
                ctx,
                widgets::Button::child_mut(&mut element).downcast(),
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
        ctx.with_id(BUTTON_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.child,
                view_state,
                ctx,
                widgets::Button::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(BUTTON_CONTENT_VIEW_ID) => self.child.message(
                view_state,
                message,
                widgets::Button::child_mut(&mut element).downcast(),
                app_state,
            ),
            None => match message.take_message::<ButtonPress>() {
                Some(press) => (self.callback)(app_state, press.button),
                None => {
                    // TODO: Panic?
                    tracing::error!(
                        "Wrong message type in Button::message: {message:?} expected {}",
                        type_name::<ButtonPress>()
                    );
                    MessageResult::Stale
                }
            },
            _ => {
                tracing::warn!("Got unexpected id path in Button::message");
                MessageResult::Stale
            }
        }
    }
}
