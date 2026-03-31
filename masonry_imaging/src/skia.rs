// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

/// Errors that can occur while creating or using a Skia renderer.
#[derive(Debug)]
pub enum Error {
    /// Creating or using the underlying `imaging_skia` renderer failed.
    Backend(imaging_skia::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "Skia render failed: {err:?}"),
        }
    }
}

impl std::error::Error for Error {}

/// Stable backend name for diagnostics.
pub const BACKEND_NAME: &str = "imaging_skia";

/// Masonry alias for the selected Skia image renderer type.
pub type Renderer = imaging_skia::SkiaRenderer;

/// Masonry alias for the selected Skia texture renderer type.
pub type TargetRenderer = imaging_skia::SkiaGpuTargetRenderer;

/// Masonry alias for the selected Skia texture target wrapper.
pub type TextureTarget<'a> = imaging_skia::TextureTarget<'a>;

/// Create a reusable headless Skia renderer.
pub fn new_headless_renderer() -> Result<Renderer, Error> {
    Ok(imaging_skia::SkiaRenderer::new())
}

/// Create a reusable GPU Skia target renderer bound to an existing WGPU adapter, device, and queue.
pub fn new_target_renderer(
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
) -> Result<TargetRenderer, Error> {
    imaging_skia::SkiaGpuTargetRenderer::new(adapter, device, queue).map_err(Error::Backend)
}
