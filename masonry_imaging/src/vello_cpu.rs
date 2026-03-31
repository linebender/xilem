// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

/// Errors that can occur while creating or using a Vello CPU renderer.
#[derive(Debug)]
pub enum Error {
    /// The underlying `imaging_vello_cpu` renderer failed.
    Backend(imaging_vello_cpu::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "Vello CPU render failed: {err:?}"),
        }
    }
}

impl std::error::Error for Error {}

/// Stable backend name for diagnostics.
pub const BACKEND_NAME: &str = "imaging_vello_cpu";

/// Masonry alias for the selected Vello CPU renderer type.
pub type Renderer = imaging_vello_cpu::VelloCpuRenderer;

/// Create a reusable headless Vello CPU renderer.
pub fn new_headless_renderer() -> Result<Renderer, Error> {
    Ok(imaging_vello_cpu::VelloCpuRenderer::new(1, 1))
}
