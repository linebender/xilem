// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::widgets::RootWidget;
use winit::window::{Window, WindowAttributes};
use xilem_core::{AnyViewState, View, ViewElement, ViewMarker};

use crate::{AnyWidgetView, ViewCtx, WindowOptions};

pub(crate) struct WindowView<State> {
    options: WindowOptions<State>,
    root_widget_view: Box<AnyWidgetView<State, ()>>,
}

impl<State> WindowView<State> {
    pub(crate) fn new(
        options: WindowOptions<State>,
        root_widget_view: Box<AnyWidgetView<State, ()>>,
    ) -> Self {
        Self {
            options,
            root_widget_view,
        }
    }
}

pub(crate) struct CreateWindow(pub WindowAttributes, pub RootWidget);

impl ViewElement for CreateWindow {
    type Mut<'a> = (&'a Window, &'a mut RenderRoot);
}

impl<State> ViewMarker for WindowView<State> where State: 'static {}

impl<State> View<State, (), ViewCtx> for WindowView<State>
where
    State: 'static,
{
    type Element = CreateWindow;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, view_state) = self.root_widget_view.build(ctx);
        let root_widget = RootWidget::from_pod(pod.into_widget_pod().erased());
        let initial_attributes = self.options.build_initial_attrs();
        (CreateWindow(initial_attributes, root_widget), view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        root_widget_view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (window, render_root): xilem_core::Mut<'_, Self::Element>,
    ) {
        self.options.rebuild(&prev.options, window);

        ctx.state_changed = true;
        self.rebuild_root_widget(prev, root_widget_view_state, ctx, render_root);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (_, render_root): xilem_core::Mut<'_, Self::Element>,
    ) {
        render_root.edit_root_widget(|mut root| {
            let mut root = root.downcast::<RootWidget>();
            self.root_widget_view.teardown(
                view_state,
                ctx,
                RootWidget::child_mut(&mut root).downcast(),
            );
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<(), xilem_core::DynMessage> {
        self.root_widget_view
            .message(view_state, id_path, message, app_state)
    }
}

impl<State> WindowView<State>
where
    State: 'static,
{
    pub(crate) fn rebuild_root_widget(
        &self,
        prev: &Self,
        root_widget_view_state: &mut AnyViewState,
        ctx: &mut ViewCtx,
        render_root: &mut RenderRoot,
    ) {
        render_root.edit_root_widget(|mut root| {
            let mut root = root.downcast::<RootWidget>();
            self.root_widget_view.rebuild(
                &prev.root_widget_view,
                root_widget_view_state,
                ctx,
                RootWidget::child_mut(&mut root).downcast(),
            );
        });
        if cfg!(debug_assertions) && !render_root.needs_rewrite_passes() {
            tracing::debug!("Widget tree didn't change as result of rebuild");
        }
    }

    pub(crate) fn on_close(&self, state: &mut State) {
        if let Some(on_close) = &self.options.callbacks.on_close {
            on_close(state);
        }
    }
}
