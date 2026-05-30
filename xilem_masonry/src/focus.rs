// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Modifiers for driving widget [text focus] from app code.
//!
//! These are exposed through the [`Focusable`] trait, which is implemented for every
//! [`WidgetView`]:
//!
//! - [`focus_on_appear`](Focusable::focus_on_appear): a sticky policy — focus the control
//!   each time it becomes visible (when it is created or un-stashed).
//! - [`focus`](Focusable::focus): an imperative, edge-triggered command — focus on the
//!   rising edge of a `bool`, give up focus on the falling edge.
//!
//! [text focus]: masonry::doc::masonry_concepts#text-focus

use std::marker::PhantomData;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// Adds focus modifiers to every [`WidgetView`].
///
/// See the [module documentation](self) for an overview.
pub trait Focusable<State: 'static, Action: 'static>: WidgetView<State, Action> + Sized {
    /// Requests [text focus] for this control each time it appears, i.e. when it is added to
    /// the tree or un-stashed (e.g. shown again by an `indexed_stack`).
    ///
    /// This is a sticky setting read at appear-time; passing `false` turns it off. If several
    /// controls request focus-on-appear in the same frame, the first one in tree (tab) order
    /// wins. The focus target is resolved to the first focusable widget in this view's
    /// subtree, so it works for wrappers like `text_input` whose focusable widget is nested.
    ///
    /// [text focus]: masonry::doc::masonry_concepts#text-focus
    fn focus_on_appear(self, on_appear: bool) -> FocusOnAppear<Self, State, Action> {
        FocusOnAppear {
            child: self,
            on_appear,
            phantom: PhantomData,
        }
    }

    /// Imperatively moves [text focus] on the *edge* of `focused`:
    ///
    /// - on the rising edge (`false` -> `true`) this control's subtree takes focus;
    /// - on the falling edge (`true` -> `false`) it gives up focus (a no-op if it isn't
    ///   currently focused).
    ///
    /// This is **edge-triggered, not level-triggered**: the initial value never fires (use
    /// [`focus_on_appear`](Self::focus_on_appear) for focus-on-load), and a steady `true`
    /// does not re-grab focus after the user tabs away. To focus again, toggle the value off
    /// and on. Typically driven from a `bool` in app state.
    ///
    /// [text focus]: masonry::doc::masonry_concepts#text-focus
    fn focus(self, focused: bool) -> Focused<Self, State, Action> {
        Focused {
            child: self,
            focused,
            phantom: PhantomData,
        }
    }
}

impl<State, Action, V> Focusable<State, Action> for V
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action> + Sized,
{
}

/// The view for [`Focusable::focus_on_appear`].
pub struct FocusOnAppear<V, State, Action> {
    child: V,
    on_appear: bool,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> ViewMarker for FocusOnAppear<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for FocusOnAppear<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<Child::Widget>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (mut child_pod, child_state) = self.child.build(ctx, app_state);
        child_pod.new_widget.options.auto_focus = self.on_appear;
        (child_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.child.rebuild(
            &prev.child,
            view_state,
            ctx,
            element.reborrow_mut(),
            app_state,
        );
        if self.on_appear != prev.on_appear {
            element.ctx.set_auto_focus(self.on_appear);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.child.message(view_state, message, element, app_state)
    }
}

/// The view for [`Focusable::focus`].
pub struct Focused<V, State, Action> {
    child: V,
    focused: bool,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> ViewMarker for Focused<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for Focused<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<Child::Widget>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // The command channel is inert at build: the initial value is not an edge.
        self.child.build(ctx, app_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.child.rebuild(
            &prev.child,
            view_state,
            ctx,
            element.reborrow_mut(),
            app_state,
        );
        if self.focused != prev.focused {
            if self.focused {
                element.ctx.focus_subtree();
            } else if element.ctx.has_focus_target() {
                // Guarded so the falling edge is a silent no-op when focus was already lost
                // (e.g. the user tabbed away), rather than warning.
                element.ctx.resign_focus();
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.child.message(view_state, message, element, app_state)
    }
}
