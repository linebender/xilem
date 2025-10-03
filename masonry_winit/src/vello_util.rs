// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple helpers for managing wgpu state and surfaces.
//!
//! This module is based on [`vello::util`](masonry_core::vello::util) module
//! with modifications for transparent surfaces.

use masonry_core::vello::{
    Error,
    wgpu::{self, MemoryBudgetThresholds},
};
use wgpu::{
    BlendComponent, BlendFactor, BlendState, CompositeAlphaMode, Device, Instance, PresentMode,
    Surface, SurfaceConfiguration, Texture, TextureFormat, TextureUsages, TextureView,
    util::{TextureBlitter, TextureBlitterBuilder},
};

/// Simple render context that maintains wgpu state for rendering the pipeline.
pub(crate) struct RenderContext {
    pub instance: Instance,
    pub devices: Vec<DeviceHandle>,
}

pub(crate) struct DeviceHandle {
    adapter: wgpu::Adapter,
    pub device: Device,
    pub queue: wgpu::Queue,
}

impl RenderContext {
    pub(crate) fn new() -> Self {
        let backends = wgpu::Backends::from_env().unwrap_or_default();
        let flags = wgpu::InstanceFlags::from_build_config().with_env();
        let backend_options = wgpu::BackendOptions::from_env_or_default();
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends,
            flags,
            backend_options,
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
        });
        Self {
            instance,
            devices: Vec::new(),
        }
    }

    /// Creates a new surface for the specified window and dimensions.
    pub(crate) async fn create_surface<'w>(
        &mut self,
        window: impl Into<wgpu::SurfaceTarget<'w>>,
        width: u32,
        height: u32,
        present_mode: PresentMode,
    ) -> Result<RenderSurface<'w>, Error> {
        self.create_render_surface(
            self.instance.create_surface(window.into())?,
            width,
            height,
            present_mode,
        )
        .await
    }

    /// Creates a new render surface for the specified window and dimensions.
    pub(crate) async fn create_render_surface<'w>(
        &mut self,
        surface: Surface<'w>,
        width: u32,
        height: u32,
        present_mode: PresentMode,
    ) -> Result<RenderSurface<'w>, Error> {
        let dev_id = self
            .device(Some(&surface))
            .await
            .ok_or(Error::NoCompatibleDevice)?;

        let device_handle = &self.devices[dev_id];
        let capabilities = surface.get_capabilities(&device_handle.adapter);
        let format = capabilities
            .formats
            .into_iter()
            .find(|it| matches!(it, TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm))
            .ok_or(Error::UnsupportedSurfaceFormat)?;

        const PREMUL_BLEND_STATE: BlendState = BlendState {
            alpha: BlendComponent::REPLACE,
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        };
        // TODO: check if the window is transparent then set alpha_mode accordingly
        // also, Opaque mode may help in saving power.
        // blocked on winit not exposing a way to check for transparency
        let (alpha_mode, blitter) = if capabilities
            .alpha_modes
            .contains(&CompositeAlphaMode::PostMultiplied)
        {
            (
                CompositeAlphaMode::PostMultiplied,
                TextureBlitter::new(&device_handle.device, format),
            )
        } else if capabilities
            .alpha_modes
            .contains(&CompositeAlphaMode::PreMultiplied)
        {
            (
                CompositeAlphaMode::PreMultiplied,
                TextureBlitterBuilder::new(&device_handle.device, format)
                    .blend_state(PREMUL_BLEND_STATE)
                    .build(),
            )
        } else {
            // TODO: check if the only available mode is Inherit then log info that postmultipled blit is being used
            // TODO: check if non-opaque base color is used on unsupported device then warn
            let texture_blitter =
                if cfg!(windows) && device_handle.adapter.get_info().name.contains("AMD") {
                    tracing::info!(
                        "on Windows with AMD GPUs use premultiplied blitting even on opaque surface"
                    );
                    TextureBlitterBuilder::new(&device_handle.device, format)
                        .blend_state(PREMUL_BLEND_STATE)
                        .build()
                } else {
                    TextureBlitter::new(&device_handle.device, format)
                };
            (CompositeAlphaMode::Auto, texture_blitter)
        };

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        let (target_texture, target_view) = create_targets(width, height, &device_handle.device);

        let surface = RenderSurface {
            surface,
            config,
            dev_id,
            format,
            target_texture,
            target_view,
            blitter,
        };
        self.configure_surface(&surface);
        Ok(surface)
    }

    /// Resizes the surface to the new dimensions.
    pub(crate) fn resize_surface(&self, surface: &mut RenderSurface<'_>, width: u32, height: u32) {
        let (texture, view) = create_targets(width, height, &self.devices[surface.dev_id].device);
        // TODO: Use clever resize semantics to avoid thrashing the memory allocator during a resize
        // especially important on metal.
        surface.target_texture = texture;
        surface.target_view = view;
        surface.config.width = width;
        surface.config.height = height;
        self.configure_surface(surface);
    }

    pub(crate) fn set_present_mode(
        &self,
        surface: &mut RenderSurface<'_>,
        present_mode: PresentMode,
    ) {
        surface.config.present_mode = present_mode;
        self.configure_surface(surface);
    }

    fn configure_surface(&self, surface: &RenderSurface<'_>) {
        let device = &self.devices[surface.dev_id].device;
        surface.surface.configure(device, &surface.config);
    }

    /// Finds or creates a compatible device handle id.
    pub(crate) async fn device(
        &mut self,
        compatible_surface: Option<&Surface<'_>>,
    ) -> Option<usize> {
        let compatible = match compatible_surface {
            Some(s) => self
                .devices
                .iter()
                .enumerate()
                .find(|(_, d)| d.adapter.is_surface_supported(s))
                .map(|(i, _)| i),
            None => (!self.devices.is_empty()).then_some(0),
        };
        if compatible.is_none() {
            return self.new_device(compatible_surface).await;
        }
        compatible
    }

    /// Creates a compatible device handle id.
    async fn new_device(&mut self, compatible_surface: Option<&Surface<'_>>) -> Option<usize> {
        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&self.instance, compatible_surface)
                .await
                .ok()?;
        let features = adapter.features();
        let limits = wgpu::Limits::default();
        let maybe_features = wgpu::Features::CLEAR_TEXTURE;
        #[cfg(feature = "tracy")]
        let maybe_features = maybe_features | wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features & maybe_features,
                required_limits: limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .ok()?;
        let device_handle = DeviceHandle {
            adapter,
            device,
            queue,
        };
        self.devices.push(device_handle);
        Some(self.devices.len() - 1)
    }
}

/// Vello uses a compute shader to render to the provided texture, which means that it can't bind the surface
/// texture in most cases.
///
/// Because of this, we need to create an "intermediate" texture which we render to, and then blit to the surface.
fn create_targets(width: u32, height: u32, device: &Device) -> (Texture, TextureView) {
    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        format: TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
    (target_texture, target_view)
}

/// Combination of surface and its configuration.
pub(crate) struct RenderSurface<'s> {
    pub surface: Surface<'s>,
    pub config: SurfaceConfiguration,
    pub dev_id: usize,
    pub format: TextureFormat,
    pub target_texture: Texture,
    pub target_view: TextureView,
    pub blitter: TextureBlitter,
}

impl std::fmt::Debug for RenderSurface<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderSurface")
            .field("surface", &self.surface)
            .field("config", &self.config)
            .field("dev_id", &self.dev_id)
            .field("format", &self.format)
            .field("target_texture", &self.target_texture)
            .field("target_view", &self.target_view)
            .field("blitter", &"(Not Debug)")
            .finish()
    }
}
