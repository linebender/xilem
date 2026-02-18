// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::kurbo::Vec2;
use masonry::widgets::{self, BadgePlacement};

use crate::core::{
    Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewId, ViewMarker, ViewPathTracker,
};
use crate::{Pod, ViewCtx, WidgetView};

/// Decorate `content` by overlaying a badge widget on top of it.
pub fn badged<State, Action, Content, BadgeV>(
    content: Content,
    badge: BadgeV,
) -> Badged<Content, BadgeV, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    Content: WidgetView<State, Action>,
    BadgeV: WidgetView<State, Action>,
{
    Badged {
        content,
        badge: Some(badge),
        placement: BadgePlacement::TopRight,
        offset: Vec2::ZERO,
        phantom: PhantomData,
    }
}

/// Like [`badged`], but the badge may be absent (for example when `count == 0`).
pub fn badged_optional<State, Action, Content, BadgeV>(
    content: Content,
    badge: Option<BadgeV>,
) -> Badged<Content, BadgeV, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    Content: WidgetView<State, Action>,
    BadgeV: WidgetView<State, Action>,
{
    Badged {
        content,
        badge,
        placement: BadgePlacement::TopRight,
        offset: Vec2::ZERO,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`badged`] / [`badged_optional`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Badged<Content, BadgeV, State, Action = ()> {
    content: Content,
    badge: Option<BadgeV>,
    placement: BadgePlacement,
    offset: Vec2,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Content, BadgeV, State, Action> Badged<Content, BadgeV, State, Action> {
    /// Sets the badge placement relative to the content.
    pub fn placement(mut self, placement: BadgePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets an additional badge offset.
    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}

const BADGED_CONTENT_VIEW_ID: ViewId = ViewId::new(0x6f3b2dfd);
const BADGED_BADGE_VIEW_ID: ViewId = ViewId::new(0x0c3f7b7a);

mod hidden {
    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct BadgedState<ContentState, BadgeState> {
        pub(crate) content: ContentState,
        pub(crate) badge: Option<BadgeState>,
    }
}

use hidden::BadgedState;

impl<Content, BadgeV, State, Action> ViewMarker for Badged<Content, BadgeV, State, Action> {}
impl<Content, BadgeV, State, Action> View<State, Action, ViewCtx>
    for Badged<Content, BadgeV, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    Content: WidgetView<State, Action>,
    BadgeV: WidgetView<State, Action>,
{
    type Element = Pod<widgets::Badged>;
    type ViewState = BadgedState<Content::ViewState, BadgeV::ViewState>;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let mut app_state = app_state;

        let (content_el, content_state) = ctx.with_id(BADGED_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.content, ctx, State::reborrow_mut(&mut app_state))
        });
        let (badge_new, badge_state) = match &self.badge {
            Some(badge_view) => {
                let (badge_el, badge_state) = ctx.with_id(BADGED_BADGE_VIEW_ID, |ctx| {
                    View::<State, Action, _>::build(
                        badge_view,
                        ctx,
                        State::reborrow_mut(&mut app_state),
                    )
                });
                (Some(badge_el.new_widget.erased()), Some(badge_state))
            }
            None => (None, None),
        };

        let widget = widgets::Badged::new_optional(content_el.new_widget, badge_new)
            .with_badge_placement(self.placement)
            .with_badge_offset(self.offset);
        (
            ctx.create_pod(widget),
            BadgedState {
                content: content_state,
                badge: badge_state,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        BadgedState { content, badge }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        if prev.placement != self.placement {
            widgets::Badged::set_badge_placement(&mut element, self.placement);
        }
        if prev.offset != self.offset {
            widgets::Badged::set_badge_offset(&mut element, self.offset);
        }

        let mut app_state = app_state;

        ctx.with_id(BADGED_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.content,
                &prev.content,
                content,
                ctx,
                widgets::Badged::content_mut(&mut element).downcast(),
                State::reborrow_mut(&mut app_state),
            );
        });

        match (&self.badge, &prev.badge) {
            (None, None) => {
                debug_assert!(badge.is_none());
            }
            (Some(badge_view), Some(prev_badge_view)) => {
                let badge_state = badge
                    .as_mut()
                    .expect("badge view state should exist when badge view exists");
                let mut badge_el = widgets::Badged::badge_mut(&mut element)
                    .expect("badge widget should exist when badge view exists");
                ctx.with_id(BADGED_BADGE_VIEW_ID, |ctx| {
                    View::<State, Action, _>::rebuild(
                        badge_view,
                        prev_badge_view,
                        badge_state,
                        ctx,
                        badge_el.downcast(),
                        State::reborrow_mut(&mut app_state),
                    );
                });
            }
            (Some(badge_view), None) => {
                let (new_badge_el, new_badge_state) = ctx.with_id(BADGED_BADGE_VIEW_ID, |ctx| {
                    View::<State, Action, _>::build(
                        badge_view,
                        ctx,
                        State::reborrow_mut(&mut app_state),
                    )
                });
                widgets::Badged::set_badge(&mut element, new_badge_el.new_widget);
                *badge = Some(new_badge_state);
            }
            (None, Some(prev_badge_view)) => {
                let prev_badge_state = badge
                    .as_mut()
                    .expect("badge view state should exist when previous badge view exists");
                let mut badge_el = widgets::Badged::badge_mut(&mut element)
                    .expect("badge widget should exist when badge view exists");
                ctx.with_id(BADGED_BADGE_VIEW_ID, |ctx| {
                    View::<State, Action, _>::teardown(
                        prev_badge_view,
                        prev_badge_state,
                        ctx,
                        badge_el.downcast(),
                    );
                });
                drop(badge_el);
                widgets::Badged::clear_badge(&mut element);
                *badge = None;
            }
        }
    }

    fn teardown(
        &self,
        BadgedState { content, badge }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(BADGED_CONTENT_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.content,
                content,
                ctx,
                widgets::Badged::content_mut(&mut element).downcast(),
            );
        });

        if let (Some(badge_view), Some(badge_state)) = (&self.badge, badge.as_mut())
            && let Some(mut badge_el) = widgets::Badged::badge_mut(&mut element)
        {
            ctx.with_id(BADGED_BADGE_VIEW_ID, |ctx| {
                View::<State, Action, _>::teardown(
                    badge_view,
                    badge_state,
                    ctx,
                    badge_el.downcast(),
                );
            });
        }
    }

    fn message(
        &self,
        BadgedState { content, badge }: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let mut app_state = app_state;
        match message.take_first() {
            Some(BADGED_CONTENT_VIEW_ID) => self.content.message(
                content,
                message,
                widgets::Badged::content_mut(&mut element).downcast(),
                State::reborrow_mut(&mut app_state),
            ),
            Some(BADGED_BADGE_VIEW_ID) => {
                let Some(badge_view) = &self.badge else {
                    tracing::error!(
                        ?message,
                        "Message arrived for badge, but badge view is None; this is a bug"
                    );
                    return MessageResult::Stale;
                };
                let Some(badge_state) = badge.as_mut() else {
                    tracing::error!(
                        ?message,
                        "Message arrived for badge, but badge state is None; this is a bug"
                    );
                    return MessageResult::Stale;
                };
                let Some(mut badge_el) = widgets::Badged::badge_mut(&mut element) else {
                    tracing::error!(
                        ?message,
                        "Message arrived for badge, but widget tree has no badge; this is a bug"
                    );
                    return MessageResult::Stale;
                };
                badge_view.message(
                    badge_state,
                    message,
                    badge_el.downcast(),
                    State::reborrow_mut(&mut app_state),
                )
            }
            None => {
                tracing::error!(
                    ?message,
                    "Message arrived in Badged::message, but Badged doesn't consume any messages, this is a bug"
                );
                MessageResult::Stale
            }
            Some(_) => {
                tracing::error!(?message, "Unexpected view id for Badged::message");
                MessageResult::Stale
            }
        }
    }
}
