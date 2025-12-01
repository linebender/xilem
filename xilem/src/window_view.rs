// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::peniko::Color;
use masonry::theme::BACKGROUND_COLOR;
use masonry_winit::app::{NewWindow, Window, WindowId};

use crate::core::{Arg, Edit, MessageCtx, Mut, View, ViewElement, ViewMarker};
use crate::{AnyWidgetView, InitialRootWidget, MasonryRoot, ViewCtx, WidgetView, WindowOptions};

/// A view representing a window.
pub struct WindowView<State: 'static> {
    pub(crate) id: WindowId,
    pub(crate) options: WindowOptions<State>,
    pub(crate) masonry_root: MasonryRoot<State>,
    /// The base color of the window.
    pub(crate) base_color: Color,
}

pub(crate) type WindowViewState = <Box<AnyWidgetView<(), ()>> as View<(), (), ViewCtx>>::ViewState;

/// A view representing a window.
///
/// `id` can be created using the [`WindowId::next()`] method and _must_ be the
/// same each frame for the same window. Usually it should be stored in app
/// state somewhere.
///
/// `title` initializes [`WindowOptions`].
pub fn window<V: WidgetView<Edit<State>>, State: 'static>(
    id: WindowId,
    title: impl Into<String>,
    root_view: V,
) -> WindowView<State> {
    WindowView {
        id,
        options: WindowOptions::new(title),
        masonry_root: MasonryRoot::new(root_view),
        base_color: BACKGROUND_COLOR,
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
    ///
    /// This is [`masonry::theme::BACKGROUND_COLOR`] by default.
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

// TODO: Reconsider how this works with ViewArgument.
// There are *reasonable* arguments for making this be `View<()>`, i.e. the root state is just another local.
impl<State> View<Edit<State>, (), ViewCtx> for WindowView<State> {
    type Element = PodWindow;

    type ViewState = WindowViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, Edit<State>>,
    ) -> (Self::Element, Self::ViewState) {
        let (InitialRootWidget(root_widget), view_state) = self.masonry_root.build(ctx, app_state);
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
        app_state: Arg<'_, Edit<State>>,
    ) {
        self.options.rebuild(&prev.options, window.handle());
        if self.base_color != prev.base_color {
            *window.base_color() = self.base_color;
        }

        self.masonry_root.rebuild(
            &prev.masonry_root,
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
        self.masonry_root
            .teardown(view_state, ctx, window.render_root());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        window: Mut<'_, Self::Element>,
        app_state: Arg<'_, Edit<State>>,
    ) -> xilem_core::MessageResult<()> {
        self.masonry_root
            .message(view_state, message, window.render_root(), app_state)
    }
}

impl<State> WindowView<State>
where
    State: 'static,
{
    pub(crate) fn on_close(&self, state: &mut State) {
        if let Some(on_close) = &self.options.callbacks.on_close {
            on_close(state);
        }
    }
}
