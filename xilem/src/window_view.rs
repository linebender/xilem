// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_winit::{app::RenderRoot, widgets::RootWidget};
use winit::window::{Window, WindowAttributes};
use xilem_core::{AnyViewState, View, ViewElement, ViewMarker};

use crate::{AnyWidgetView, ViewCtx, WindowAttrs};

pub(crate) struct WindowView<State> {
    pub(crate) attributes: WindowAttrs<State>,
    root_widget_view: Box<AnyWidgetView<State, ()>>,
}

impl<State> WindowView<State> {
    pub(crate) fn new(
        attributes: WindowAttrs<State>,
        root_widget_view: Box<AnyWidgetView<State, ()>>,
    ) -> Self {
        Self {
            attributes,
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
        let initial_attributes = self.attributes.build_initial_attrs();
        (CreateWindow(initial_attributes, root_widget), view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        root_widget_view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (window, render_root): xilem_core::Mut<'_, Self::Element>,
    ) {
        self.rebuild_reactive_window_attributes(prev, window);
        self.warn_for_changed_initial_attributes(prev);

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
}

impl<State> WindowView<State> {
    fn rebuild_reactive_window_attributes(&self, prev: &Self, window: &Window) {
        let current = &self.attributes.reactive;
        let prev = &prev.attributes.reactive;

        if current.title != prev.title {
            window.set_title(&current.title);
        }
        if current.resizable != prev.resizable {
            window.set_resizable(current.resizable);
        }
        if current.cursor != prev.cursor {
            window.set_cursor(current.cursor.clone());
        }
        if current.min_inner_size != prev.min_inner_size {
            window.set_min_inner_size(current.min_inner_size);
        }
        if current.max_inner_size != prev.max_inner_size {
            window.set_max_inner_size(current.max_inner_size);
        }
    }

    fn warn_for_changed_initial_attributes(&self, prev: &Self) {
        let current = &self.attributes.initial;
        let prev = &prev.attributes.initial;

        if current.inner_size != prev.inner_size {
            tracing::warn!(
                "attempted to change inner_size attribute after window creation, this is not supported"
            );
        }
        if current.position != prev.position {
            tracing::warn!(
                "attempted to change position attribute after window creation, this is not supported"
            );
        }
        if current.window_icon.is_some() != prev.window_icon.is_some() {
            tracing::warn!(
                "attempted to change window_icon attribute after window creation, this is not supported"
            );
        }
    }
}
