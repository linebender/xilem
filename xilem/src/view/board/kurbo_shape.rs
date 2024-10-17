// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use masonry::widget::{KurboShape, SvgElement};
use vello::kurbo;
use xilem_core::{DynMessage, MessageResult, Mut, OrphanView};

use crate::{Pod, ViewCtx, WidgetView};

pub trait GraphicsView<State, Action = ()>: WidgetView<State, Action, Widget: SvgElement> {}

impl<V, State, Action> GraphicsView<State, Action> for V where
    V: WidgetView<State, Action, Widget: SvgElement> + Send + Sync
{
}

macro_rules! impl_orphan_view {
    ($t:ident) => {
        impl<State: 'static, Action: 'static> OrphanView<kurbo::$t, State, Action, DynMessage>
            for ViewCtx
        {
            type OrphanViewState = ();
            type OrphanElement = Pod<KurboShape>;

            fn orphan_build(
                view: &kurbo::$t,
                _ctx: &mut ViewCtx,
            ) -> (Self::OrphanElement, Self::OrphanViewState) {
                (Pod::new(KurboShape::new(view.clone())), ())
            }

            fn orphan_rebuild<'el>(
                new: &kurbo::$t,
                prev: &kurbo::$t,
                (): &mut Self::OrphanViewState,
                ctx: &mut ViewCtx,
                mut element: Mut<'el, Self::OrphanElement>,
            ) -> Mut<'el, Self::OrphanElement> {
                if new != prev {
                    element.set_shape(new.clone().into());
                    ctx.mark_changed();
                }
                element
            }

            fn orphan_teardown(
                _view: &kurbo::$t,
                (): &mut Self::OrphanViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Self::OrphanElement>,
            ) {
            }

            fn orphan_message(
                _view: &kurbo::$t,
                (): &mut Self::OrphanViewState,
                _id_path: &[xilem_core::ViewId],
                message: DynMessage,
                _app_state: &mut State,
            ) -> MessageResult<Action, DynMessage> {
                MessageResult::Stale(message)
            }
        }
    };
}

impl_orphan_view!(PathSeg);
impl_orphan_view!(Arc);
impl_orphan_view!(BezPath);
impl_orphan_view!(Circle);
impl_orphan_view!(CircleSegment);
impl_orphan_view!(CubicBez);
impl_orphan_view!(Ellipse);
impl_orphan_view!(Line);
impl_orphan_view!(QuadBez);
impl_orphan_view!(Rect);
impl_orphan_view!(RoundedRect);
