use std::num::NonZeroUsize;
use std::sync::Arc;

use tracing::warn;
use vello::util::{RenderContext, RenderSurface};
use vello::{peniko::Color, AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::PresentMode;
use winit::dpi::PhysicalPosition;
use winit::error::EventLoopError;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::app_driver::{AppDriver, DriverCtx};
use crate::event::{PointerState, WindowEvent};
use crate::render_root::{self, RenderRoot, WindowSizePolicy};
use crate::{PointerEvent, TextEvent, Widget};

pub struct EventLoopRunner {
    window: Arc<Window>,
    event_loop: EventLoop<()>,
    render_root: RenderRoot,
    app_driver: Box<dyn AppDriver>,
}

struct MainState<'a> {
    window: Arc<Window>,
    render_cx: RenderContext,
    surface: RenderSurface<'a>,
    renderer: Option<Renderer>,
    pointer_state: PointerState,
    app_driver: Box<dyn AppDriver>,
}

impl EventLoopRunner {
    pub fn new(
        root_widget: impl Widget,
        window: Window,
        event_loop: EventLoop<()>,
        app_driver: impl AppDriver + 'static,
    ) -> Self {
        Self {
            window: Arc::new(window),
            event_loop,
            render_root: RenderRoot::new(root_widget, WindowSizePolicy::User),
            app_driver: Box::new(app_driver),
        }
    }

    pub fn run(self) -> Result<(), EventLoopError> {
        let mut render_cx = RenderContext::new().unwrap();
        let size = self.window.inner_size();
        let surface = pollster::block_on(render_cx.create_surface(
            self.window.clone(),
            size.width,
            size.height,
            PresentMode::AutoVsync,
        ))
        .unwrap();
        let mut render_root = self.render_root;
        let mut main_state = MainState {
            window: self.window,
            render_cx,
            surface,
            renderer: None,
            pointer_state: PointerState::empty(),
            app_driver: self.app_driver,
        };

        self.event_loop.run(move |event, window_target| {
            if let winit::event::Event::WindowEvent { event: e, .. } = event {
                match e {
                    WinitWindowEvent::RedrawRequested => {
                        let scene = render_root.redraw();
                        main_state.render(scene);
                    }
                    WinitWindowEvent::CloseRequested => window_target.exit(),
                    WinitWindowEvent::Resized(size) => {
                        render_root.handle_window_event(WindowEvent::Resize(size));
                    }
                    WinitWindowEvent::ModifiersChanged(modifiers) => {
                        render_root.handle_text_event(TextEvent::ModifierChange(modifiers.state()));
                    }
                    WinitWindowEvent::CursorMoved { position, .. } => {
                        main_state.pointer_state.position = position;
                        render_root.handle_pointer_event(PointerEvent::PointerMove(
                            main_state.pointer_state.clone(),
                        ));
                    }
                    WinitWindowEvent::CursorLeft { .. } => {
                        render_root.handle_pointer_event(PointerEvent::PointerLeave(
                            main_state.pointer_state.clone(),
                        ));
                    }
                    WinitWindowEvent::MouseInput { state, button, .. } => match state {
                        winit::event::ElementState::Pressed => {
                            render_root.handle_pointer_event(PointerEvent::PointerDown(
                                button,
                                main_state.pointer_state.clone(),
                            ));
                        }
                        winit::event::ElementState::Released => {
                            render_root.handle_pointer_event(PointerEvent::PointerUp(
                                button,
                                main_state.pointer_state.clone(),
                            ));
                        }
                    },
                    WinitWindowEvent::MouseWheel { delta, .. } => {
                        let delta = match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                            winit::event::MouseScrollDelta::PixelDelta(delta) => (delta.x, delta.y),
                        };
                        let delta = PhysicalPosition::new(delta.0, delta.1);
                        render_root.handle_pointer_event(PointerEvent::MouseWheel(
                            delta,
                            main_state.pointer_state.clone(),
                        ));
                    }
                    _ => (),
                }
                main_state.process_signals(&mut render_root);
            }
        })
    }
}

impl MainState<'_> {
    fn render(&mut self, scene: Scene) {
        //let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width;
        let height = size.height;

        if self.surface.config.width != width || self.surface.config.height != height {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
        }

        #[cfg(FALSE)]
        let transform = if scale != 1.0 {
            Some(Affine::scale(scale))
        } else {
            None
        };

        let Ok(surface_texture) = self.surface.surface.get_current_texture() else {
            warn!("failed to acquire next swapchain texture");
            return;
        };
        let dev_id = self.surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(self.surface.format),
            use_cpu: false,
            antialiasing_support: AaSupport {
                area: true,
                msaa8: false,
                msaa16: false,
            },
            num_init_threads: NonZeroUsize::new(1),
        };
        let render_params = RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .get_or_insert_with(|| Renderer::new(device, renderer_options).unwrap())
            .render_to_surface(device, queue, &scene, &surface_texture, &render_params)
            .expect("failed to render to surface");
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);
    }

    fn process_signals(&mut self, render_root: &mut RenderRoot) {
        while let Some(signal) = render_root.pop_signal() {
            match signal {
                render_root::RenderRootSignal::Action(action, widget_id) => {
                    render_root.edit_root_widget(|root| {
                        let mut driver_ctx = DriverCtx {
                            main_root_widget: root,
                        };
                        self.app_driver
                            .on_action(&mut driver_ctx, widget_id, action);
                    });
                }
                render_root::RenderRootSignal::TextFieldAdded => {
                    // TODO
                }
                render_root::RenderRootSignal::TextFieldRemoved => {
                    // TODO
                }
                render_root::RenderRootSignal::TextFieldFocused => {
                    // TODO
                }
                render_root::RenderRootSignal::ImeStarted => {
                    // TODO
                }
                render_root::RenderRootSignal::ImeMoved => {
                    // TODO
                }
                render_root::RenderRootSignal::ImeInvalidated => {
                    // TODO
                }
                render_root::RenderRootSignal::RequestRedraw => {
                    self.window.request_redraw();
                }
                render_root::RenderRootSignal::RequestAnimFrame => {
                    // TODO
                    self.window.request_redraw();
                }
                render_root::RenderRootSignal::SpawnWorker(_worker_fn) => {
                    // TODO
                }
                render_root::RenderRootSignal::TakeFocus => {
                    self.window.focus_window();
                }
                render_root::RenderRootSignal::SetCursor(cursor_icon) => {
                    self.window.set_cursor_icon(cursor_icon);
                }
                render_root::RenderRootSignal::SetSize(size) => {
                    // TODO - Handle return value?
                    let _ = self.window.request_inner_size(size);
                }
                render_root::RenderRootSignal::SetTitle(title) => {
                    self.window.set_title(&title);
                }
            }
        }
    }
}
