// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::type_name;

pub use masonry::core::PointerButton;
use masonry::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, HoveredBorderColor, Padding,
};
use masonry::widgets::{self, ButtonPress};

use crate::core::{MessageContext, Mut, View, ViewMarker, ViewPathTracker};
use crate::property_tuple::PropertyTuple;
use crate::style::Style;
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
        properties: ButtonProps::default(),
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
        properties: ButtonProps::default(),
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
    properties: ButtonProps,
}

impl<F> Button<F> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

const LABEL_VIEW_ID: ViewId = ViewId::new(0);

impl<F> Style for Button<F> {
    type Props = ButtonProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    pub ButtonProps;
    Button<F>;

    Background, 0;
    BorderColor, 1;
    BorderWidth, 2;
    BoxShadow, 3;
    CornerRadius, 4;
    Padding, 5;
    ActiveBackground, 6;
    DisabledBackground, 7;
    HoveredBorderColor, 8;
);

impl<F> ViewMarker for Button<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State, Option<PointerButton>) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widgets::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, ()) = ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.label, ctx, app_state)
        });
        ctx.with_leaf_action_widget(|ctx| {
            let mut pod = ctx.create_pod(widgets::Button::new(child.new_widget));
            pod.new_widget.properties = self.properties.build_properties();
            pod.new_widget.options.disabled = self.disabled;
            pod
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        if element.ctx.is_explicitly_disabled() != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label,
                &prev.label,
                state,
                ctx,
                widgets::Button::child_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label,
                &mut (),
                ctx,
                widgets::Button::child_mut(&mut element).downcast(),
                app_state,
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(LABEL_VIEW_ID) => self.label.message(
                &mut (),
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
