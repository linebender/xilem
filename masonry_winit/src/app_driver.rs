// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::hash::Hash;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use masonry_core::app::RenderRoot;
use masonry_core::core::{ErasedAction, WidgetId};
use masonry_core::peniko::ImageData;
use masonry_core::vello::wgpu;
use tracing::field::DisplayValue;
use winit::event_loop::ActiveEventLoop;

use crate::app::MasonryState;
use crate::event_loop_runner::{NewWindow, Window};

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

/// Access to Masonry's WGPU device state.
///
/// This is provided via [`AppDriver::on_wgpu_ready`] so applications can create GPU resources
/// (textures, pipelines, etc.) using the same `Device`/`Queue` as Masonry.
pub struct WgpuContext<'a> {
    /// The WGPU instance used by Masonry.
    pub instance: &'a wgpu::Instance,
    /// The WGPU adapter used to create the device.
    pub adapter: &'a wgpu::Adapter,
    /// The shared WGPU device.
    pub device: &'a wgpu::Device,
    /// The shared WGPU queue.
    pub queue: &'a wgpu::Queue,
}

/// Strategy for selecting `wgpu::Limits` when requesting the WGPU device.
#[derive(Clone, Debug, Default)]
pub enum WgpuLimits {
    /// Use `wgpu::Limits::default()`.
    #[default]
    Default,
    /// Use `adapter.limits()` (maximum supported by the selected adapter).
    Adapter,
    /// Use the provided limits.
    Custom(wgpu::Limits),
}

/// A trait for defining how your app interacts with the Masonry widget tree.
///
/// When launching your app with [`crate::app::run`], you need to provide
/// a type that implements this trait.
#[expect(unused_variables, reason = "Default impls doesn't use arguments")]
pub trait AppDriver {
    /// A hook which will be executed when a widget emits an `action`.
    ///
    /// This action is type-erased, and the type of action emitted will depend on.
    /// Each widget should document which types of action it might emit.
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
    ///
    /// There are circumstances under which this will be called multiple times during the lifecycle of your app.
    /// This is not intended to be the behaviour of Masonry Winit long-term, but this method should currently
    /// not assume it will only be called once (but should feel free to waste work if it is called multiple times,
    /// for example, as the mentioned circumstances are very rare).
    // TODO: Turn into something like on window created, or split into two.
    fn on_start(&mut self, state: &mut MasonryState<'_>) {}

    /// A hook called when a user has requested to close a window.
    fn on_close_requested(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        ctx.exit();
    }

    /// Called when Masonry has created its WGPU device.
    fn on_wgpu_ready(&mut self, _wgpu: &WgpuContext<'_>) {}
}

impl DriverCtx<'_, '_> {
    // TODO - Add method to create timer

    /// Access the [`RenderRoot`] of the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found.
    pub fn render_root(&mut self, window_id: WindowId) -> &mut RenderRoot {
        &mut self.window(window_id).render_root
    }

    /// Access the [`Window`] state of the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found.
    pub fn window(&mut self, window_id: WindowId) -> &mut Window {
        self.state.window_mut(window_id)
    }

    /// Creates a new window.
    ///
    /// # Panics
    ///
    /// Panics if the window id is already used by another window.
    pub fn create_window(&mut self, new_window: NewWindow) {
        self.state.create_window(self.event_loop, new_window);
    }

    /// Closes the given window.
    ///
    /// # Panics
    ///
    /// Panics if the window cannot be found.
    pub fn close_window(&mut self, window_id: WindowId) {
        self.state.close_window(window_id);
    }

    /// Set a persistent Vello image override.
    ///
    /// This associates the given [`ImageData`] with the provided GPU texture.
    ///
    /// Correct behaviour is not guaranteed if the texture does not have the same
    /// dimensions as the image.
    ///
    /// Overrides persist until cleared with [`DriverCtx::clear_image_override`].
    ///
    /// Note: Masonry currently uses a shared Vello renderer, so overrides are global to that
    /// renderer/device.
    ///
    /// ## When does this take effect?
    ///
    /// The underlying Vello [`Renderer`](masonry_core::vello::Renderer) is created lazily during
    /// rendering. If you call this method before the renderer exists, Masonry will store the
    /// override and apply it automatically once a renderer has been created.
    ///
    /// # Texture requirements
    ///
    /// When set, Vello will copy from `texture` into its internal image atlas whenever the
    /// `image` is drawn in the UI scene.
    ///
    /// The texture must be `Rgba8Unorm` and include `COPY_SRC` usage.
    pub fn set_image_override(&mut self, image: ImageData, texture: wgpu::Texture) {
        self.state.set_image_override(image, texture);
    }

    /// Clear a previously-set image override for the given `ImageData`.
    ///
    /// Note: overrides are global to the current renderer/device; see [`set_image_override`](Self::set_image_override).
    pub fn clear_image_override(&mut self, image: &ImageData) {
        self.state.clear_image_override(image);
    }

    /// Exits the application (stops the event loop).
    pub fn exit(&mut self) {
        self.state.exit = true;
    }
}
