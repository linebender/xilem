// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

pub use masonry_winit::core::PointerButton;
use masonry_winit::widgets;
use xilem_core::ViewPathTracker;

use crate::core::{DynMessage, Mut, View, ViewMarker};
use crate::view::Label;
use crate::{MessageResult, Pod, ViewCtx, ViewId};

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
///
/// # Examples
/// To use button provide it with a button text and a closure.
/// ```ignore
/// use xilem::view::button;
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
/// button("Button", |state: &mut State| {
///      state.increase();
/// })
/// ```
///
/// Create a `button` with a custom `label`.
///
/// ```ignore
/// use xilem::view::{button, label};
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
/// let label = label("Button").weight(FontWeight::BOLD);
///
/// button(label, |state: &mut State| {
///     state.increase();
/// })
/// ```
pub fn button<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button<
    impl for<'a> Fn(&'a mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
> {
    Button {
        label: label.into(),
        callback: move |state: &mut State, button| match button {
            None | Some(PointerButton::Primary) => MessageResult::Action(callback(state)),
            _ => MessageResult::Nop,
        },
        disabled: false,
    }
}

/// A button which calls `callback` when pressed.
pub fn button_any_pointer<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State, Option<PointerButton>) -> Action + Send + 'static,
) -> Button<
    impl for<'a> Fn(&'a mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
> {
    Button {
        label: label.into(),
        callback: move |state: &mut State, button| MessageResult::Action(callback(state, button)),
        disabled: false,
    }
}

/// The [`View`] created by [`button`] from a `label` and a callback.
///
/// See `button` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button<F> {
    // N.B. This widget is *implemented* to handle any kind of view with an element
    // type of `Label` even though it currently does not do so.
    label: Label,
    callback: F,
    disabled: bool,
}

impl<F> Button<F> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

const LABEL_VIEW_ID: ViewId = ViewId::new(0);

impl<F> ViewMarker for Button<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State, Option<PointerButton>) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widgets::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (child, ()) = ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.label, ctx)
        });
        ctx.with_leaf_action_widget(|ctx| {
            let mut pod = ctx.new_pod(widgets::Button::from_label_pod(child.into_widget_pod()));
            pod.options.disabled = self.disabled;
            pod
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if element.ctx.is_disabled() != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label,
                &prev.label,
                state,
                ctx,
                widgets::Button::label_mut(&mut element),
            );
        });
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label,
                &mut (),
                ctx,
                widgets::Button::label_mut(&mut element),
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match id_path.split_first() {
            Some((&LABEL_VIEW_ID, rest)) => self.label.message(&mut (), rest, message, app_state),
            None => match message.downcast::<masonry_winit::core::Action>() {
                Ok(action) => {
                    if let masonry_winit::core::Action::ButtonPressed(button) = *action {
                        (self.callback)(app_state, button)
                    } else {
                        tracing::error!("Wrong action type in Button::message: {action:?}");
                        MessageResult::Stale(DynMessage(action))
                    }
                }
                Err(message) => {
                    tracing::error!("Wrong message type in Button::message: {message:?}");
                    MessageResult::Stale(message)
                }
            },
            _ => {
                tracing::warn!("Got unexpected id path in Button::message");
                MessageResult::Stale(message)
            }
        }
    }
}
