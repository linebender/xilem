// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::widgets::Passthrough;
use xilem_core::Edit;

use crate::core::{Arg, MessageCtx, Mut, View, ViewElement, ViewMarker};
use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};

/// A view representing a [`RenderRoot`].
pub struct RenderRootView<State: 'static> {
    /// The view generating the `RenderRoot`'s contents.
    pub(crate) root_widget_view: Box<AnyWidgetView<Edit<State>, ()>>,
}

pub(crate) type RenderRootViewState =
    <Box<AnyWidgetView<(), ()>> as View<(), (), ViewCtx>>::ViewState;

/// A wrapper type around [`Passthrough`] for implementing [`ViewElement`].
pub struct InitialRootWidget(pub Pod<Passthrough>);

impl ViewElement for InitialRootWidget {
    type Mut<'a> = &'a mut RenderRoot;
}

impl<State: 'static> RenderRootView<State> {
    /// Create the view from the [`WidgetView`] representing its root widget.
    pub fn new(root_view: impl WidgetView<Edit<State>>) -> Self {
        Self {
            root_widget_view: root_view.boxed(),
        }
    }
}

impl<State> ViewMarker for RenderRootView<State> where State: 'static {}
impl<State> View<Edit<State>, (), ViewCtx> for RenderRootView<State> {
    type Element = InitialRootWidget;

    type ViewState = RenderRootViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, Edit<State>>,
    ) -> (Self::Element, Self::ViewState) {
        let (root_widget, view_state) = self.root_widget_view.build(ctx, app_state);
        (InitialRootWidget(root_widget), view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        root_widget_view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        render_root: Mut<'_, Self::Element>,
        app_state: Arg<'_, Edit<State>>,
    ) {
        let mut root_id = None;
        render_root.edit_base_layer(|mut root| {
            let mut root = root.downcast();
            self.root_widget_view.rebuild(
                &prev.root_widget_view,
                root_widget_view_state,
                ctx,
                root.reborrow_mut(),
                app_state,
            );
            root_id = Some(root.widget.inner_id());
        });
        render_root.set_focus_fallback(root_id);
        if cfg!(debug_assertions) && !render_root.needs_rewrite_passes() {
            tracing::debug!("Widget tree didn't change as result of rebuild");
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        render_root: Mut<'_, Self::Element>,
    ) {
        render_root.edit_base_layer(|mut root| {
            self.root_widget_view
                .teardown(view_state, ctx, root.downcast());
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        render_root: Mut<'_, Self::Element>,
        app_state: Arg<'_, Edit<State>>,
    ) -> xilem_core::MessageResult<()> {
        render_root.edit_base_layer(|mut root| {
            self.root_widget_view
                .message(view_state, message, root.downcast(), app_state)
        })
    }
}
