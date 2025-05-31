// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::hash::Hash;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use masonry::core::Widget;
use tracing::field::DisplayValue;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use crate::app::{MasonryState, RenderRoot};
use crate::core::{Action, WidgetId};
use crate::event_loop_runner::{WindowState, WindowStatus};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct WindowId(pub(crate) NonZeroU64);

impl WindowId {
    /// Allocate a new, unique `WindowId`.
    ///
    /// You must ensure that a given `WindowId` is only ever used for one
    /// window at a time.
    pub fn next() -> Self {
        static WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id.try_into().unwrap())
    }

    /// A serialized representation of the `WindowId` for debugging purposes.
    pub fn trace(self) -> DisplayValue<NonZeroU64> {
        tracing::field::display(self.0)
    }
}

/// Context for the [`AppDriver`] trait.
pub struct DriverCtx<'a, 's> {
    pub(crate) state: &'a mut MasonryState<'s>,
    pub(crate) event_loop: &'a ActiveEventLoop,
}

/// A trait for defining how your app interacts with the Masonry widget tree.
///
/// When launching your app with [`crate::app::run`], you need to provide
/// a type that implements this trait.
#[expect(unused_variables, reason = "Default impls doesn't use arguments")]
pub trait AppDriver {
    /// A hook which will be executed when a widget emits an [`Action`].
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: Action,
    );

    /// A hook which will be executed when the application starts, to allow initial configuration of the `MasonryState`.
    ///
    /// Use cases include loading fonts.
    fn on_start(&mut self, state: &mut MasonryState<'_>) {}

    /// A hook called on application startup to create the initial windows.
    fn create_initial_windows(&mut self, ctx: &mut DriverCtx<'_, '_>);

    /// A hook called when a user has requested to close a window.
    fn on_close_requested(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        ctx.exit();
    }
}

impl DriverCtx<'_, '_> {
    // TODO - Add method to create timer

    /// Access the [`RenderRoot`] of the given window.
    pub fn render_root(&mut self, window_id: WindowId) -> &mut RenderRoot {
        &mut self.state.window_state_mut(window_id).render_root
    }

    /// Access the [`Window`] handle of the given window.
    pub fn window_handle(&self, window_id: WindowId) -> &Window {
        let WindowStatus::Rendering { window, .. } = &self.state.window_state(window_id).status
        else {
            panic!("window is not not in rendering state");
        };
        window
    }

    /// Access the [`Window`] handle and [`RenderRoot`] of the given window.
    pub fn window_handle_and_render_root(
        &mut self,
        window_id: WindowId,
    ) -> (&Window, &mut RenderRoot) {
        let state = self.state.window_state_mut(window_id);
        let WindowStatus::Rendering { window, .. } = &state.status else {
            panic!("window is not not in rendering state");
        };
        (window, &mut state.render_root)
    }

    /// Creates a new window.
    pub fn create_window(
        &mut self,
        window_id: WindowId,
        root_widget: impl Widget,
        attributes: WindowAttributes,
    ) {
        let state = WindowState::new(
            window_id,
            Box::new(root_widget),
            attributes.clone(),
            self.state.signal_sender.clone(),
            self.state.default_properties.clone(),
        );
        self.state.create_window(self.event_loop, state, attributes);
    }

    /// Closes the given window.
    pub fn close_window(&mut self, window_id: WindowId) {
        self.state.close_window(window_id);
    }

    /// Exits the application (stops the event loop).
    pub fn exit(&mut self) {
        self.state.exit = true;
    }
}
