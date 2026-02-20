// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widgets;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A view which puts `child` into a scrollable region.
///
/// This corresponds to the Masonry [`Portal`](masonry::widgets::Portal) widget.
pub fn portal<Child, State, Action>(child: Child) -> Portal<Child, State, Action>
where
    State: 'static,
    Child: WidgetView<State, Action>,
{
    Portal {
        child,
        constrain_horizontal: false,
        constrain_vertical: false,
        must_fill: false,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`portal`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Portal<V, State, Action> {
    child: V,
    constrain_horizontal: bool,
    constrain_vertical: bool,
    must_fill: bool,
    phantom: PhantomData<fn(State) -> Action>,
}

impl<V, State, Action> Portal<V, State, Action> {
    /// Set horizontal constraining of the child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its width. The child can be as wide as it wants,
    ///   and the viewport gets moved around to see all of it.
    /// - When it is `true`, the [`Portal`]'s width will be passed down as an upper bound
    ///   on the width of the child. There will be no horizontal scrollbar and
    ///   the mouse wheel can't be used to horizontally scroll either.
    pub fn constrain_horizontal(mut self, constrain_horizontal: bool) -> Self {
        self.constrain_horizontal = constrain_horizontal;
        self
    }

    /// Sets vertical constraining of the child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its height. The child can be as tall as it wants,
    ///   and the viewport gets moved around to see all of it.
    /// - When it is `true`, the [`Portal`]'s height will be passed down as an upper bound
    ///   on the height of the child. There will be no vertical scrollbar and
    ///   the mouse wheel can't be used to vertically scroll either.
    pub fn constrain_vertical(mut self, constrain_vertical: bool) -> Self {
        self.constrain_vertical = constrain_vertical;
        self
    }

    /// Sets whether the child must fill the view.
    ///
    /// If `true`, the child size is guaranteed to be at least the size of the portal.
    pub fn must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
        self
    }
}

impl<V, State, Action> ViewMarker for Portal<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for Portal<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<widgets::Portal<Child::Widget>>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // The Portal `View` doesn't get any messages directly (yet - scroll events?), so doesn't need to
        // use ctx.with_id.
        let (child, child_state) = self.child.build(ctx, app_state);
        let widget_pod = ctx.create_pod(
            widgets::Portal::new(child.new_widget)
                .constrain_horizontal(self.constrain_horizontal)
                .constrain_vertical(self.constrain_vertical)
                .content_must_fill(self.must_fill),
        );
        (widget_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if self.constrain_horizontal != prev.constrain_horizontal {
            widgets::Portal::set_constrain_horizontal(&mut element, self.constrain_horizontal);
        }
        if self.constrain_vertical != prev.constrain_vertical {
            widgets::Portal::set_constrain_vertical(&mut element, self.constrain_vertical);
        }
        if self.must_fill != prev.must_fill {
            widgets::Portal::set_content_must_fill(&mut element, self.must_fill);
        }

        let child_element = widgets::Portal::child_mut(&mut element);
        self.child
            .rebuild(&prev.child, view_state, ctx, child_element, app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let child_element = widgets::Portal::child_mut(&mut element);
        self.child.teardown(view_state, ctx, child_element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let child_element = widgets::Portal::child_mut(&mut element);
        self.child
            .message(view_state, message, child_element, app_state)
    }
}
