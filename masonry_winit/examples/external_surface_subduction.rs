// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates an `ExternalSurface` realized through `subduction`.
//!
//! This example composites Masonry scene layers and an animated host-managed `wgpu`
//! viewport into the same window surface. The overlay caption inside the slot is a normal
//! Masonry widget rendered above the external viewport.

#![cfg_attr(not(test), windows_subsystem = "windows")]

use std::collections::HashMap;

use masonry::accesskit::{Node, Role};
use masonry::app::{ExternalVisualLayer, VisualLayerPlan};
use masonry::core::{
    AccessCtx, ChildrenIds, ErasedAction, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PropertiesRef, RegisterCtx, Widget, WidgetPod,
};
use masonry::dpi::PhysicalSize;
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size as KurboSize, Stroke};
use masonry::layout::{AsUnit, LenReq, UnitPoint};
use masonry::peniko::Color;
use masonry::properties::Padding;
use masonry::theme::default_property_set;
use masonry::widgets::{ChildAlignment, ExternalSurface, Flex, Label, SizedBox, ZStack};
use wgpu::StoreOp;

use masonry_winit::app::{
    AppDriver, DriverCtx, ExternalLayerRenderResult, ExternalLayerRenderer, ExternalLayerTarget,
    NewWindow, PresentVisualLayersResult, PresentationTarget, SubductionPresenter,
    VelloSceneLayerRenderer, WindowId,
};
use masonry_winit::winit::dpi::LogicalSize;
use masonry_winit::winit::window::Window;

const TITLE: &str = "External Surface Subduction Demo";
const SLOT_SIZE: (f64, f64) = (640.0, 360.0);
const OVERLAY_SIZE: (f64, f64) = (300.0, 48.0);
struct Driver {
    compositor: SubductionPresenter,
    scene_renderer: VelloSceneLayerRenderer,
    demo_renderer: DemoRenderer,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        _window_id: WindowId,
        _ctx: &mut DriverCtx<'_, '_>,
        _widget_id: masonry::core::WidgetId,
        _action: ErasedAction,
    ) {
    }

    fn present_visual_layers(
        &mut self,
        _window_id: WindowId,
        target: PresentationTarget<'_>,
        layers: &VisualLayerPlan,
    ) -> PresentVisualLayersResult {
        match self.compositor.present(
            target,
            layers,
            &mut self.scene_renderer,
            &mut self.demo_renderer,
        ) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("subduction compositor demo failed: {err}");
                PresentVisualLayersResult::NotHandled
            }
        }
    }
}

fn scale_rect(rect: Rect, scale_factor: f64) -> Rect {
    Rect::new(
        rect.x0 * scale_factor,
        rect.y0 * scale_factor,
        rect.x1 * scale_factor,
        rect.y1 * scale_factor,
    )
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "scissor coordinates are clamped into the valid output texture extent"
)]
fn rect_to_scissor(rect: Rect, output_size: PhysicalSize<u32>) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect.x0.floor().max(0.0).min(f64::from(output_size.width));
    let y0 = rect.y0.floor().max(0.0).min(f64::from(output_size.height));
    let x1 = rect.x1.ceil().max(0.0).min(f64::from(output_size.width));
    let y1 = rect.y1.ceil().max(0.0).min(f64::from(output_size.height));
    let width = (x1 - x0).max(0.0) as u32;
    let height = (y1 - y0).max(0.0) as u32;
    (width > 0 && height > 0).then_some((x0 as u32, y0 as u32, width, height))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "demo shader uniforms intentionally narrow window-space values to f32 for GPU upload"
)]
fn as_f32(value: f64) -> f32 {
    value as f32
}

struct FramedSlot {
    child: WidgetPod<dyn Widget>,
    border: Color,
    fill: Color,
    corner_radius: f64,
}

impl FramedSlot {
    fn new(child: NewWidget<impl Widget + ?Sized>, border: Color, fill: Color) -> Self {
        Self {
            child: child.erased().to_pod(),
            border,
            fill,
            corner_radius: 0.0,
        }
    }

    fn with_corner_radius(mut self, corner_radius: f64) -> Self {
        self.corner_radius = corner_radius;
        self
    }
}

impl Widget for FramedSlot {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        ctx.redirect_measurement(&mut self.child, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: KurboSize) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        ctx.derive_baselines(&self.child);
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let rect = ctx.content_box();
        let shape = rect.to_rounded_rect(self.corner_radius);
        painter.fill(shape, self.fill).draw();
        painter
            .stroke(
                rect.inset(-1.0).to_rounded_rect(self.corner_radius + 1.0),
                &Stroke::new(2.0),
                self.border,
            )
            .draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

struct DemoRenderer {
    start_time: std::time::Instant,
    pipelines: HashMap<wgpu::TextureFormat, DemoPipeline>,
}

struct DemoPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
}

impl Default for DemoRenderer {
    fn default() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            pipelines: HashMap::new(),
        }
    }
}

impl DemoRenderer {
    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        output_size: PhysicalSize<u32>,
        slot_bounds: Rect,
        clip_bounds: Option<Rect>,
    ) {
        let pipeline = self
            .pipelines
            .entry(format)
            .or_insert_with(|| Self::create_pipeline(device, format));

        let elapsed = self.start_time.elapsed().as_secs_f32();
        queue.write_buffer(
            &pipeline.uniforms,
            0,
            &uniform_bytes(elapsed, output_size, slot_bounds),
        );

        let scissor = rect_to_scissor(clip_bounds.unwrap_or(slot_bounds), output_size);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("subduction demo viewport encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("subduction demo viewport pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some((x, y, width, height)) = scissor {
                pass.set_scissor_rect(x, y, width, height);
            }
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &pipeline.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }

    fn create_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> DemoPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("subduction demo shader"),
            source: wgpu::ShaderSource::Wgsl(DEMO_SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("subduction demo bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        wgpu::BufferSize::new(32).expect("uniform buffer size should be non-zero"),
                    ),
                },
                count: None,
            }],
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("subduction demo uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("subduction demo bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("subduction demo pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("subduction demo pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        DemoPipeline {
            pipeline,
            bind_group,
            uniforms,
        }
    }
}

impl ExternalLayerRenderer for DemoRenderer {
    fn render_external_layer(
        &mut self,
        target: ExternalLayerTarget<'_>,
        layer: ExternalVisualLayer,
    ) -> Result<ExternalLayerRenderResult, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.render(
            target.device,
            target.queue,
            target.view,
            target.format,
            target.output_size,
            scale_rect(layer.window_bounds(), target.scale_factor),
            layer
                .window_clip_bounds()
                .map(|rect| scale_rect(rect, target.scale_factor)),
        );
        Ok(ExternalLayerRenderResult {
            request_redraw: true,
        })
    }
}

fn uniform_bytes(time: f32, output_size: PhysicalSize<u32>, slot_bounds: Rect) -> [u8; 32] {
    let values = [
        time,
        output_size.width as f32,
        output_size.height as f32,
        0.0,
        as_f32(slot_bounds.x0),
        as_f32(slot_bounds.y0),
        as_f32(slot_bounds.width()),
        as_f32(slot_bounds.height()),
    ];
    let mut bytes = [0_u8; 32];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..(index + 1) * 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

const DEMO_SHADER: &str = r#"
struct DemoUniforms {
    time_and_size: vec4<f32>,
    slot: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: DemoUniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.14, 0.18, 0.24);
    let b = vec3<f32>(0.44, 0.29, 0.22);
    let c = vec3<f32>(0.55, 0.52, 0.42);
    let d = vec3<f32>(0.95, 0.84, 0.55);
    return a + b * cos(6.28318 * (c * t + d));
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let time = uniforms.time_and_size.x;
    let slot = uniforms.slot;
    let slot_uv = (frag_pos.xy - slot.xy) / slot.zw;
    let aspect = slot.z / max(slot.w, 1.0);
    var z = vec2<f32>(
        (slot_uv.x - 0.5) * aspect * 2.4,
        (slot_uv.y - 0.5) * 2.4
    );
    let c = vec2<f32>(
        -0.79 + 0.11 * sin(time * 0.19),
         0.16 + 0.08 * cos(time * 0.13)
    );

    var iter: u32 = 0u;
    var smooth_iter = 0.0;
    loop {
        if (iter >= 96u || dot(z, z) > 64.0) {
            break;
        }
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        iter = iter + 1u;
    }

    if (iter < 96u) {
        let mag = length(z);
        smooth_iter = f32(iter) + 1.0 - log2(log2(max(mag, 1.0001)));
    } else {
        smooth_iter = f32(iter);
    }

    let glow = clamp(1.0 - length(slot_uv - vec2<f32>(0.5, 0.5)) * 1.4, 0.0, 1.0);
    let t = smooth_iter / 96.0;
    let color = palette(t * 0.85 + time * 0.02) + vec3<f32>(0.12, 0.08, 0.03) * glow;
    return vec4<f32>(color, 1.0);
}
"#;

fn make_widget_tree() -> NewWidget<impl Widget> {
    let slot = NewWidget::new(
        SizedBox::new(NewWidget::new(
            ZStack::new()
                .with(
                    NewWidget::new(FramedSlot::new(
                        ExternalSurface::new()
                            .with_alt_text("Animated wgpu viewport composited by subduction")
                            .with_auto_id(),
                        Color::from_rgb8(0x57, 0x97, 0xb8),
                        Color::from_rgba8(0x57, 0x97, 0xb8, 0x20),
                    )),
                    ChildAlignment::ParentAligned,
                )
                .with(
                    NewWidget::new(
                        SizedBox::new(NewWidget::new(
                            FramedSlot::new(
                                Label::new("Masonry overlay above the compositor layer")
                                    .with_props(Padding::from_vh(10.0, 16.0)),
                                Color::from_rgba8(0xe7, 0xf1, 0xf8, 0xcc),
                                Color::from_rgba8(0x0d, 0x14, 0x1b, 0x88),
                            )
                            .with_corner_radius(14.0),
                        ))
                        .size(OVERLAY_SIZE.0.px(), OVERLAY_SIZE.1.px()),
                    ),
                    UnitPoint::TOP_LEFT,
                ),
        ))
        .size(SLOT_SIZE.0.px(), SLOT_SIZE.1.px()),
    );

    Flex::column()
        .with_fixed(NewWidget::new(Label::new(
            "Subduction compositor: a host-managed wgpu viewport inside the widget tree",
        )))
        .with_fixed_spacer(14.0_f64.px())
        .with_fixed(slot)
        .with_fixed_spacer(12.0_f64.px())
        .with_fixed(NewWidget::new(Label::new(
            "The caption inside the slot is a normal Masonry scene layer composited above the external surface.",
        )))
        .with_auto_id()
}

fn main() {
    let window_attributes = Window::default_attributes()
        .with_title(TITLE)
        .with_inner_size(LogicalSize::new(760.0, 520.0))
        .with_min_inner_size(LogicalSize::new(520.0, 420.0));

    masonry_winit::app::run(
        vec![NewWindow::new(
            window_attributes,
            make_widget_tree().erased(),
        )],
        Driver {
            compositor: SubductionPresenter::new(),
            scene_renderer: VelloSceneLayerRenderer::new(),
            demo_renderer: DemoRenderer::default(),
        },
        default_property_set(),
    )
    .unwrap();
}
