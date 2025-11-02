// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::{FocusFallbackPolicy, RenderRoot};
use masonry::peniko::Color;
use masonry_winit::app::{NewWindow, Window, WindowId};

use crate::core::{AnyViewState, MessageContext, Mut, View, ViewElement, ViewMarker};
use crate::{AnyWidgetView, ViewCtx, WidgetView, WindowOptions};

/// A view representing a window.
pub struct WindowView<State> {
    pub(crate) id: WindowId,
    options: WindowOptions<State>,
    root_widget_view: Box<AnyWidgetView<State, ()>>,
    /// The base color of the window.
    base_color: Color,
}

/// A view representing a window.
///
/// `id` can be created using the [`WindowId::next()`] method and _must_ be the
/// same each frame for the same window. Usually it should be stored in app
/// state somewhere.
///
/// `title` initializes [`WindowOptions`].
pub fn window<V: WidgetView<State>, State: 'static>(
    id: WindowId,
    title: impl Into<String>,
    root_view: V,
) -> WindowView<State> {
    WindowView {
        id,
        options: WindowOptions::new(title),
        root_widget_view: root_view.boxed(),
        base_color: Color::BLACK,
    }
}

impl<State> WindowView<State> {
    /// Modify window options in-place.
    pub fn with_options(
        mut self,
        f: impl FnOnce(WindowOptions<State>) -> WindowOptions<State>,
    ) -> Self {
        self.options = f(self.options);
        self
    }

    /// Set base color of the window.
    pub fn with_base_color(mut self, color: Color) -> Self {
        self.base_color = color;
        self
    }
}

/// A newtype wrapper around [`NewWindow`] for implementing [`ViewElement`].
pub struct PodWindow(pub NewWindow);

impl ViewElement for PodWindow {
    type Mut<'a> = &'a mut Window;
}

impl<State> ViewMarker for WindowView<State> where State: 'static {}

impl<State> View<State, (), ViewCtx> for WindowView<State>
where
    State: 'static,
{
    type Element = PodWindow;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (root_widget, view_state) = self.root_widget_view.build(ctx, app_state);
        let initial_attributes = self.options.build_initial_attrs();
        (
            PodWindow(
                NewWindow::new_with_id(
                    self.id,
                    initial_attributes,
                    root_widget.new_widget.erased(),
                )
                .with_base_color(self.base_color),
            ),
            view_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        root_widget_view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        window: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.options.rebuild(&prev.options, window.handle());
        if self.base_color != prev.base_color {
            *window.base_color() = self.base_color;
        }

        ctx.set_state_changed(true);
        self.rebuild_root_widget(
            prev,
            root_widget_view_state,
            ctx,
            window.render_root(),
            app_state,
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        window: Mut<'_, Self::Element>,
    ) {
        window.render_root().edit_base_layer(|mut root| {
            self.root_widget_view
                .teardown(view_state, ctx, root.downcast());
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        window: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<()> {
        window.render_root().edit_base_layer(|mut root| {
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
        render_root.edit_base_layer(|mut root| {
            let mut root = root.downcast();
            self.root_widget_view.rebuild(
                &prev.root_widget_view,
                root_widget_view_state,
                ctx,
                root.reborrow_mut(),
                app_state,
            );
        });
        // Provide a sensible default fallback for Xilem apps: first text input in the tree.
        let _ = render_root.set_focus_fallback_policy(FocusFallbackPolicy::FirstTextInput);
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
