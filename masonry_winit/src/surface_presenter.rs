// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Internal presentation helpers for Masonry Winit.
//!
//! This module owns the last-mile host/presenter policy inside `masonry_winit`:
//! blitting the rendered target texture into the platform surface.
//!
//! It does not own widget paint semantics, backend rendering, visual-layer adaptation, or
//! window-event orchestration.

use std::sync::Arc;

use winit::window::Window as WindowHandle;

use crate::vello_util::RenderSurface;

/// Blit the rendered intermediate target into the platform surface and present it.
pub(crate) fn present_surface(
    surface: &RenderSurface<'_>,
    surface_texture: wgpu::SurfaceTexture,
    window: &Arc<WindowHandle>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Surface Blit"),
    });
    surface.blitter.copy(
        device,
        &mut encoder,
        &surface.target_view,
        &surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default()),
    );
    queue.submit([encoder.finish()]);
    window.pre_present_notify();
    surface_texture.present();
    {
        let _render_poll_span =
            tracing::info_span!("Waiting for GPU to finish rendering").entered();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    }
}
