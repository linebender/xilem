// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::ArcStr;
use masonry::widgets;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A non-interactive badge (pill) widget that hosts a single child.
pub fn badge<State, Action, V>(child: V) -> Badge<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    Badge {
        child,
        phantom: PhantomData,
    }
}

/// A badge containing a styled text label.
pub fn badge_text(text: impl Into<ArcStr>) -> BadgeText {
    BadgeText { text: text.into() }
}

/// A badge displaying a numeric count.
///
/// When `max_count` is `Some(max)`, numbers greater than `max` display as `max+`.
/// When `max_count` is `None`, Masonry's default overflow formatting (`99+`) is used.
pub fn badge_count(count: u32, max_count: Option<u32>) -> BadgeCount {
    let overflow = match max_count {
        Some(max) => widgets::BadgeCountOverflow::Cap {
            max,
            show_plus: true,
        },
        None => widgets::BadgeCountOverflow::default(),
    };
    BadgeCount { count, overflow }
}

/// Like [`badge_count`], but returns `None` if `count == 0`.
pub fn badge_count_nonzero(count: u32) -> Option<BadgeCount> {
    (count != 0).then(|| badge_count(count, None))
}

/// The [`View`] created by [`badge`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Badge<V, State, Action = ()> {
    child: V,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> ViewMarker for Badge<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx> for Badge<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widgets::Badge>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.child.build(ctx, app_state);
        (
            ctx.create_pod(widgets::Badge::new(child.new_widget)),
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
        let mut child = widgets::Badge::child_mut(&mut element);
        self.child
            .rebuild(&prev.child, view_state, ctx, child.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::Badge::child_mut(&mut element);
        self.child.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::Badge::child_mut(&mut element);
        self.child
            .message(view_state, message, child.downcast(), app_state)
    }
}

/// The [`View`] created by [`badge_text`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct BadgeText {
    text: ArcStr,
}

impl ViewMarker for BadgeText {}
impl<State: 'static, Action> View<State, Action, ViewCtx> for BadgeText {
    type Element = Pod<widgets::Badge>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        (
            ctx.create_pod(widgets::Badge::with_text(self.text.clone())),
            (),
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.text != self.text {
            let mut child = widgets::Badge::child_mut(&mut element);
            let mut label = child.downcast::<widgets::Label>();
            widgets::Label::set_text(&mut label, self.text.clone());
        }
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in BadgeText::message, but BadgeText doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}

/// The [`View`] created by [`badge_count`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct BadgeCount {
    count: u32,
    overflow: widgets::BadgeCountOverflow,
}

impl BadgeCount {
    /// Sets the overflow formatting for the count.
    pub fn overflow(mut self, overflow: widgets::BadgeCountOverflow) -> Self {
        self.overflow = overflow;
        self
    }
}

fn format_count(count: u32, overflow: widgets::BadgeCountOverflow) -> ArcStr {
    match overflow {
        widgets::BadgeCountOverflow::Exact => count.to_string().into(),
        widgets::BadgeCountOverflow::Cap { max, show_plus } => {
            if count > max {
                if show_plus {
                    format!("{max}+").into()
                } else {
                    max.to_string().into()
                }
            } else {
                count.to_string().into()
            }
        }
    }
}

impl ViewMarker for BadgeCount {}
impl<State: 'static, Action> View<State, Action, ViewCtx> for BadgeCount {
    type Element = Pod<widgets::Badge>;
    type ViewState = ArcStr;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let text = format_count(self.count, self.overflow);
        (
            ctx.create_pod(widgets::Badge::with_text(text.clone())),
            text,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        text: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.count == self.count && prev.overflow == self.overflow {
            return;
        }

        *text = format_count(self.count, self.overflow);
        let mut child = widgets::Badge::child_mut(&mut element);
        let mut label = child.downcast::<widgets::Label>();
        widgets::Label::set_text(&mut label, text.clone());
    }

    fn teardown(
        &self,
        _text: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        &self,
        _text: &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in BadgeCount::message, but BadgeCount doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}
