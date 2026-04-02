// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use crate::masonry::widgets;
use crate::view::Label;
use crate::{Pod, ViewCtx, WidgetView};

/// Returns collapsible panel with a header that contains a child widget.
///
/// Defaults to collapsed.
pub fn collapse_panel<State: 'static, Action, V: WidgetView<State, Action>>(
    header: Label,
    child: V,
) -> CollapsePanel<State, Action, V> {
    CollapsePanel {
        child,
        collapsed: true,
        header,

        phantom: PhantomData,
    }
}

/// A collapsible panel with a header that contains a child widget.
pub struct CollapsePanel<State, Action, V> {
    child: V,
    collapsed: bool,
    header: Label,

    phantom: PhantomData<fn(State) -> Action>,
}

impl<State, Action, V> CollapsePanel<State, Action, V> {
    /// Set the collapsed state of the widget.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` leads to the wrong view.
/// This is a randomly generated 32 bit number - 3776130728 in decimal.
const COLLAPSE_PANEL_HEADER_VIEW_ID: ViewId = ViewId::new(0xE1132EA8);

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` leads to the wrong view.
/// This is a randomly generated 32 bit number - 922419961 in decimal.
const COLLAPSE_PANEL_CONTENT_VIEW_ID: ViewId = ViewId::new(0x36FB02F9);

impl<State, Action, V> ViewMarker for CollapsePanel<State, Action, V> {}
impl<State, Action, V> View<State, Action, ViewCtx> for CollapsePanel<State, Action, V>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widgets::CollapsePanel>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (header, _) = ctx.with_id(COLLAPSE_PANEL_HEADER_VIEW_ID, |ctx| {
            View::<State, (), _>::build(&self.header, ctx, app_state)
        });
        let (child, child_state) = ctx.with_id(COLLAPSE_PANEL_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.child, ctx, app_state)
        });
        let pod = ctx.create_pod(widgets::CollapsePanel::from_label(
            self.collapsed,
            header.new_widget,
            child.new_widget,
        ));
        (pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.collapsed != self.collapsed {
            widgets::CollapsePanel::set_collapsed(&mut element, self.collapsed);
        }
        ctx.with_id(COLLAPSE_PANEL_HEADER_VIEW_ID, |ctx| {
            View::<State, (), _>::rebuild(
                &self.header,
                &prev.header,
                &mut (),
                ctx,
                widgets::CollapsePanel::header_label_mut(&mut element).downcast(),
                app_state,
            );
        });
        ctx.with_id(COLLAPSE_PANEL_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.child,
                &prev.child,
                state,
                ctx,
                widgets::CollapsePanel::child_mut(&mut element).downcast(),
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
        ctx.with_id(COLLAPSE_PANEL_HEADER_VIEW_ID, |ctx| {
            View::<State, (), _>::teardown(
                &self.header,
                &mut (),
                ctx,
                widgets::CollapsePanel::header_label_mut(&mut element).downcast(),
            );
        });
        ctx.with_id(COLLAPSE_PANEL_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.child,
                view_state,
                ctx,
                widgets::CollapsePanel::child_mut(&mut element).downcast(),
            );
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(COLLAPSE_PANEL_HEADER_VIEW_ID) => self.header.message(
                &mut (),
                message,
                widgets::CollapsePanel::header_label_mut(&mut element).downcast(),
                app_state,
            ),
            Some(COLLAPSE_PANEL_CONTENT_VIEW_ID) => self.child.message(
                view_state,
                message,
                widgets::CollapsePanel::child_mut(&mut element).downcast(),
                app_state,
            ),
            None => {
                tracing::error!(
                    ?message,
                    "Message arrived in CollapsePanel::message, \
					but CollapsePanel doesn't consume any messages, this is a bug"
                );
                MessageResult::Stale
            }
            _ => {
                tracing::warn!("Got unexpected id path in CollapsePanel::message");
                MessageResult::Stale
            }
        }
    }
}
