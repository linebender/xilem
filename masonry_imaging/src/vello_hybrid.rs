// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

use crate::headless_wgpu;

/// Errors that can occur while creating or using a Vello Hybrid renderer.
#[derive(Debug)]
pub enum Error {
    /// Headless WGPU initialization failed.
    Init,
    /// The underlying `imaging_vello_hybrid` renderer failed.
    Backend(imaging_vello_hybrid::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => write!(f, "Vello Hybrid renderer initialization failed"),
            Self::Backend(err) => write!(f, "Vello Hybrid render failed: {err:?}"),
        }
    }
}

impl std::error::Error for Error {}

/// Stable backend name for diagnostics.
pub const BACKEND_NAME: &str = "imaging_vello_hybrid";

/// Masonry alias for the selected Vello Hybrid renderer type.
pub type Renderer = imaging_vello_hybrid::VelloHybridRenderer;

/// Masonry alias for the selected Vello Hybrid texture renderer type.
pub type TargetRenderer = imaging_vello_hybrid::VelloHybridTargetRenderer;

/// Masonry alias for the selected Vello Hybrid texture target wrapper.
pub type TextureTarget<'a> = imaging_vello_hybrid::TextureTarget<'a>;

/// Create a reusable headless Vello Hybrid renderer.
pub fn new_headless_renderer() -> Result<Renderer, Error> {
    let (device, queue) = headless_wgpu::try_init_device_and_queue().map_err(|_| Error::Init)?;
    Ok(imaging_vello_hybrid::VelloHybridRenderer::new(
        device, queue,
    ))
}

/// Create a reusable Vello Hybrid target renderer bound to an existing WGPU device and queue.
pub fn new_target_renderer(device: wgpu::Device, queue: wgpu::Queue) -> TargetRenderer {
    imaging_vello_hybrid::VelloHybridTargetRenderer::new(device, queue)
}
