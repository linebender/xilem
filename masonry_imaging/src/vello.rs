// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

use crate::headless_wgpu;

/// Errors that can occur while creating or using a Vello renderer.
#[derive(Debug)]
pub enum Error {
    /// Headless WGPU initialization failed.
    Init,
    /// The underlying `imaging_vello` renderer failed.
    Backend(imaging_vello::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => write!(f, "Vello renderer initialization failed"),
            Self::Backend(err) => write!(f, "Vello render failed: {err:?}"),
        }
    }
}

impl std::error::Error for Error {}

/// Stable backend name for diagnostics.
pub const BACKEND_NAME: &str = "imaging_vello";

/// Masonry alias for the selected Vello renderer type.
pub type Renderer = imaging_vello::VelloRenderer;

/// Masonry alias for the selected Vello texture renderer type.
pub type TargetRenderer = imaging_vello::VelloRenderer;

/// Masonry alias for the selected Vello texture target wrapper.
pub type TextureTarget = imaging_wgpu::TextureViewTarget;

/// Create a reusable headless Vello renderer.
pub fn new_headless_renderer() -> Result<Renderer, Error> {
    let (device, queue) = headless_wgpu::try_init_device_and_queue().map_err(|_| Error::Init)?;
    imaging_vello::VelloRenderer::new(device, queue).map_err(Error::Backend)
}

/// Create a reusable Vello target renderer bound to an existing WGPU device and queue.
pub fn new_target_renderer(
    device: wgpu::Device,
    queue: wgpu::Queue,
) -> Result<TargetRenderer, Error> {
    imaging_vello::VelloRenderer::new(device, queue).map_err(Error::Backend)
}
