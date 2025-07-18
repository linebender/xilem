// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::hash::Hash;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use masonry_core::app::RenderRoot;
use masonry_core::core::{ErasedAction, Widget, WidgetId, WidgetPod};
use tracing::field::DisplayValue;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window as WindowHandle, WindowAttributes};

use crate::app::MasonryState;
use crate::event_loop_runner::WindowState;

/// A unique and persistent identifier for a window.
///
/// [`MasonryState`] internally maps these to winit window ids ([`winit::window::WindowId`]).
/// Applications should only use this struct and not be concerned with the winit window ids.
/// When the application is suspended and resumed this id will stay the same, while the
/// winit window id will change.
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
    state: &'a mut MasonryState<'s>,
    event_loop: &'a ActiveEventLoop,
}

impl<'a, 's> DriverCtx<'a, 's> {
    pub(crate) fn new(state: &'a mut MasonryState<'s>, event_loop: &'a ActiveEventLoop) -> Self {
        Self { state, event_loop }
    }
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
        action: ErasedAction,
    );

    /// A hook which will be executed when the application starts, to allow initial configuration of the `MasonryState`.
    ///
    /// Use cases include loading fonts.
    fn on_start(&mut self, state: &mut MasonryState<'_>) {}

    /// A hook called when a user has requested to close a window.
    fn on_close_requested(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        ctx.exit();
    }
}

impl DriverCtx<'_, '_> {
    // TODO - Add method to create timer

    /// Access the [`RenderRoot`] of the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found.
    pub fn render_root(&mut self, window_id: WindowId) -> &mut RenderRoot {
        &mut self.state.window_mut(window_id).render_root
    }

    /// Access the [`WindowHandle`] of the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found or the window is not in the rendering state.
    pub fn window_handle(&self, window_id: WindowId) -> &WindowHandle {
        let window = self.state.window(window_id);
        let WindowState::Rendering { handle: window, .. } = &window.state else {
            panic!(
                "window with id {window_id:?} is in {:?} state, expected Rendering state",
                window.state
            );
        };
        window
    }

    /// Access the [`WindowHandle`] and [`RenderRoot`] of the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found or the window is not in the rendering state.
    pub fn window_handle_and_render_root(
        &mut self,
        window_id: WindowId,
    ) -> (&WindowHandle, &mut RenderRoot) {
        let window = self.state.window_mut(window_id);
        let WindowState::Rendering { handle, .. } = &window.state else {
            panic!(
                "window with id {window_id:?} is in {:?} state, expected Rendering state",
                window.state
            );
        };
        (handle, &mut window.render_root)
    }

    /// Creates a new window.
    ///
    /// # Panics
    ///
    /// Panics if the window id is already used by another window.
    pub fn create_window(
        &mut self,
        window_id: WindowId,
        attributes: WindowAttributes,
        root_widget: WidgetPod<dyn Widget>,
    ) {
        self.state
            .create_window(self.event_loop, window_id, attributes, root_widget);
    }

    /// Closes the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found.
    pub fn close_window(&mut self, window_id: WindowId) {
        self.state.close_window(window_id);
    }

    /// Exits the application (stops the event loop).
    pub fn exit(&mut self) {
        self.state.exit = true;
    }
}
