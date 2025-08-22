// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget};
use winit::window::{Window, WindowAttributes};

use crate::core::{AnyViewState, MessageContext, Mut, View, ViewElement, ViewMarker};
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

pub(crate) struct CreateWindow(pub WindowAttributes, pub NewWidget<dyn Widget>);

impl ViewElement for CreateWindow {
    type Mut<'a> = (&'a dyn Window, &'a mut RenderRoot);
}

impl<State> ViewMarker for WindowView<State> where State: 'static {}

impl<State> View<State, (), ViewCtx> for WindowView<State>
where
    State: 'static,
{
    type Element = CreateWindow;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (root_widget, view_state) = self.root_widget_view.build(ctx, app_state);
        let initial_attributes = self.options.build_initial_attrs();
        (
            CreateWindow(initial_attributes, root_widget.new_widget.erased()),
            view_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        root_widget_view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (window, render_root): Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.options.rebuild(&prev.options, window);

        ctx.set_state_changed(true);
        self.rebuild_root_widget(prev, root_widget_view_state, ctx, render_root, app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (_, render_root): Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        render_root.edit_root_widget(|mut root| {
            self.root_widget_view
                .teardown(view_state, ctx, root.downcast(), app_state);
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        (_, render_root): Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<()> {
        render_root.edit_root_widget(|mut root| {
            self.root_widget_view
                .message(view_state, message, root.downcast(), app_state)
        })
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
        app_state: &mut State,
    ) {
        let mut root_id = None;
        render_root.edit_root_widget(|mut root| {
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

    pub(crate) fn on_close(&self, state: &mut State) {
        if let Some(on_close) = &self.options.callbacks.on_close {
            on_close(state);
        }
    }
}
