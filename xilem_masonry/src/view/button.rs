// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::type_name;
use std::marker::PhantomData;

use masonry::core::ArcStr;
pub use masonry::core::PointerButton;
use masonry::widgets::{self, ButtonPress};

use crate::core::{
    MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker,
};
use crate::view::{Label, label};
use crate::{Pod, ViewCtx, WidgetView};

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
///
/// `child` will be the button's contents. and should be a non-interactive widget, such as a
/// [`label`](label::label), or a layout widget containing several non-interactive widgets.
/// This avoids cases where an inner interactive widget "steals" mouse focus from the outer
/// widget, or is inadvertently impossible to interactive with.
///
/// For making a button with default text styling directly from a string, you can
/// use [`text_button`] as a shorthand for `button(label(text), callback)`.
///
/// The button can also be activated using the keyboard when it has the keyboard focus.
/// Currently this happens when <kbd>Space</kbd> or <kbd>↵ Enter</kbd> are pressed, and is not configurable.
/// If you need to handle middle and right clicks on the button, as well as separate handling for
/// touch, you can use [`button_any_pointer`].
///
/// # Examples
///
/// To create a simple button with styled text:
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::masonry::parley::style::FontWeight;
/// use xilem::view::{button, label};
/// # use xilem::WidgetView;
/// # use xilem::core::Edit;
///
/// struct State {
///     count: i32,
/// }
///
/// # fn view() -> impl WidgetView<Edit<State>> {
/// let label = label("Increase").weight(FontWeight::BOLD);
///
/// button(label, |state: &mut State| {
///     state.count += 1;
/// })
/// # }
/// ```
///
/// To create a button with more complex (non-interactive) contents children:
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::masonry::parley::style::FontWeight;
/// use xilem::view::{button, label, flex_row, FlexExt};
/// # use xilem::WidgetView;
/// # type State = u32;
/// # use xilem::core::Edit;
///
/// # fn view() -> impl WidgetView<Edit<State>> {
/// let children = flex_row((
///     label("👍").flex(1.0),
///     label("Like").weight(FontWeight::BOLD),
/// ));
///
/// button(children, |_: &mut State| {})
/// # }
/// ```
pub fn button<
    State: 'static,
    Action,
    V: WidgetView<State, Action>,
    F: Fn(&mut State) -> Action + Send + 'static,
>(
    child: V,
    callback: F,
) -> Button<
    State,
    Action,
    impl Fn(&mut State, Option<PointerButton>) -> MessageResult<Action> + Send + 'static,
    V,
> {
    Button {
        child,
        callback: move |state: &mut State, button| match button {
            None | Some(PointerButton::Primary) => MessageResult::Action(callback(state)),
            _ => MessageResult::Nop,
        },
        disabled: false,
        phantom: PhantomData,
    }
}

/// A button with default styled text.
///
/// This is equivalent to `button(label(text), callback)`, and is useful for
/// making buttons quickly from string literals.
/// For more advanced text styling, prefer [`button`].
pub fn text_button<State: 'static, Action>(
    text: impl Into<ArcStr>,
    callback: impl Fn(&mut State) -> Action + Send + Sync + 'static,
) -> Button<
    State,
    Action,
    impl Fn(&mut State, Option<PointerButton>) -> MessageResult<Action>
    + Send
    + Sync
    + 'static,
    Label,
> {
    button(label(text), callback)
}

/// A button which calls `callback` when pressed with any mouse button, providing
/// the specific mouse button.
///
/// Note that the callback may be called with `None` as the pointer, which indicates
/// that the button was activated with the keyboard or a touch screen (see also [`ButtonPress`]).
/// There is not currently any support for detecting when <kbd>≣ Menu</kbd> was pressed
/// (so as to treat that as a right click, for example).
/// Similarly, there is not currently long-press support.
///
/// For more documentation and examples, see [`button`].
pub fn button_any_pointer<State: 'static, Action, V: WidgetView<State, Action>>(
    child: V,
    callback: impl Fn(&mut State, Option<PointerButton>) -> Action + Send + Sync + 'static,
) -> Button<
    State,
    Action,
    impl Fn(&mut State, Option<PointerButton>) -> MessageResult<Action>
    + Send
    + Sync
    + 'static,
    V,
> {
    Button {
        child,
        callback: move |state: &mut State, button| {
            MessageResult::Action(callback(state, button))
        },
        disabled: false,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`button`].
///
/// See `button`'s documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button<State, Action, F, V> {
    child: V,
    callback: F,
    disabled: bool,
    phantom: PhantomData<fn(State) -> Action>,
}

impl<State, Action, F, V> Button<State, Action, F, V> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` leads to the wrong view.
/// This is a randomly generated 32 bit number - 3502359742 in decimal.
const BUTTON_CONTENT_VIEW_ID: ViewId = ViewId::new(0xd0c1c4be);

impl<State, Action, F, V> ViewMarker for Button<State, Action, F, V> {}
impl<F, V, State, Action> View<State, Action, ViewCtx> for Button<State, Action, F, V>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
    F: Fn(&mut State, Option<PointerButton>) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widgets::Button>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut State,
    ) -> (Self::Element, Self::ViewState) {
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
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
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
