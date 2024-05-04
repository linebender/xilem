// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::num::NonZeroUsize;
use std::sync::Arc;

use accesskit_winit::Adapter;
use tracing::subscriber::SetGlobalDefaultError;
use tracing::{debug, warn};
use vello::kurbo::Affine;
use vello::util::{RenderContext, RenderSurface};
use vello::{peniko::Color, AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::PresentMode;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalPosition;
use winit::error::EventLoopError;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app_driver::{AppDriver, DriverCtx};
use crate::event::{PointerState, WindowEvent};
use crate::render_root::{self, RenderRoot, WindowSizePolicy};
use crate::{PointerEvent, TextEvent, Widget};

struct MainState<'a> {
    window: Arc<Window>,
    render_cx: RenderContext,
    surface: RenderSurface<'a>,
    render_root: RenderRoot,
    renderer: Option<Renderer>,
    pointer_state: PointerState,
    app_driver: Box<dyn AppDriver>,
    accesskit_adapter: Adapter,
}

pub fn run(
    window_attributes: WindowAttributes,
    root_widget: impl Widget,
    app_driver: impl AppDriver + 'static,
) -> Result<(), EventLoopError> {
    let visible = window_attributes.visible;
    let window_attributes = window_attributes.with_visible(false);

    let event_loop = EventLoop::with_user_event().build()?;
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attributes).unwrap();

    let event_loop_proxy = event_loop.create_proxy();
    let adapter = Adapter::with_event_loop_proxy(&window, event_loop_proxy);
    window.set_visible(visible);

    run_with(window, event_loop, adapter, root_widget, app_driver)
}

pub fn run_with(
    window: Window,
    event_loop: EventLoop<accesskit_winit::Event>,
    accesskit_adapter: Adapter,
    root_widget: impl Widget,
    app_driver: impl AppDriver + 'static,
) -> Result<(), EventLoopError> {
    let window = Arc::new(window);
    let mut render_cx = RenderContext::new().unwrap();
    let size = window.inner_size();
    let surface = pollster::block_on(render_cx.create_surface(
        window.clone(),
        size.width,
        size.height,
        PresentMode::AutoVsync,
    ))
    .unwrap();
    let scale_factor = window.scale_factor();
    let mut main_state = MainState {
        window,
        render_cx,
        surface,
        render_root: RenderRoot::new(root_widget, WindowSizePolicy::User, scale_factor),
        renderer: None,
        pointer_state: PointerState::empty(),
        app_driver: Box::new(app_driver),
        accesskit_adapter,
    };

    // If there is no default tracing subscriber, we set our own. If one has
    // already been set, we get an error which we swallow.
    // By now, we're about to take control of the event loop. The user is unlikely
    // to try to set their own subscriber once the event loop has started.
    let _ = try_init_tracing();

    event_loop.run_app(&mut main_state)
}

impl ApplicationHandler<accesskit_winit::Event> for MainState<'_> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // FIXME: initialize window in this handler because initializing it before running the event loop is deprecated
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WinitWindowEvent) {
        self.accesskit_adapter.process_event(&self.window, &event);

        match event {
            WinitWindowEvent::RedrawRequested => {
                let scene = self.render_root.redraw();
                self.render(scene);
            }
            WinitWindowEvent::CloseRequested => event_loop.exit(),
            WinitWindowEvent::Resized(size) => {
                self.render_root
                    .handle_window_event(WindowEvent::Resize(size));
            }
            WinitWindowEvent::ModifiersChanged(modifiers) => {
                self.pointer_state.mods = modifiers;
                self.render_root
                    .handle_text_event(TextEvent::ModifierChange(modifiers.state()));
            }
            WinitWindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.render_root.handle_text_event(TextEvent::KeyboardKey(
                    event,
                    self.pointer_state.mods.state(),
                ));
            }
            WinitWindowEvent::Ime(ime) => {
                self.render_root.handle_text_event(TextEvent::Ime(ime));
            }
            WinitWindowEvent::Focused(new_focus) => {
                self.render_root
                    .handle_text_event(TextEvent::FocusChange(new_focus));
            }
            WinitWindowEvent::CursorMoved { position, .. } => {
                self.pointer_state.physical_position = position;
                self.pointer_state.position = position.to_logical(self.window.scale_factor());
                self.render_root
                    .handle_pointer_event(PointerEvent::PointerMove(self.pointer_state.clone()));
            }
            WinitWindowEvent::CursorLeft { .. } => {
                self.render_root
                    .handle_pointer_event(PointerEvent::PointerLeave(self.pointer_state.clone()));
            }
            WinitWindowEvent::MouseInput { state, button, .. } => match state {
                winit::event::ElementState::Pressed => {
                    self.render_root
                        .handle_pointer_event(PointerEvent::PointerDown(
                            button,
                            self.pointer_state.clone(),
                        ));
                }
                winit::event::ElementState::Released => {
                    self.render_root
                        .handle_pointer_event(PointerEvent::PointerUp(
                            button,
                            self.pointer_state.clone(),
                        ));
                }
            },
            WinitWindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        LogicalPosition::new(x as f64, y as f64)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(delta) => {
                        delta.to_logical(self.window.scale_factor())
                    }
                };
                self.render_root
                    .handle_pointer_event(PointerEvent::MouseWheel(
                        delta,
                        self.pointer_state.clone(),
                    ));
            }
            _ => (),
        }

        while let Some(signal) = self.render_root.pop_signal() {
            match signal {
                render_root::RenderRootSignal::Action(action, widget_id) => {
                    self.render_root.edit_root_widget(|root| {
                        debug!("Action {:?} on widget {:?}", action, widget_id);
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
                render_root::RenderRootSignal::SetCursor(cursor) => {
                    self.window.set_cursor(cursor);
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

        self.accesskit_adapter
            .update_if_active(|| self.render_root.root_accessibility(false));
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: accesskit_winit::Event) {
        match event.window_event {
            // Note that this event can be called at any time, even multiple times if
            // the user restarts their screen reader.
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                self.accesskit_adapter
                    .update_if_active(|| self.render_root.root_accessibility(true));
            }
            accesskit_winit::WindowEvent::ActionRequested(action_request) => {
                self.render_root.root_on_access_event(action_request);
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }
}

impl MainState<'_> {
    fn render(&mut self, scene: Scene) {
        let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width;
        let height = size.height;

        if self.surface.config.width != width || self.surface.config.height != height {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
        }

        let transformed_scene = if scale == 1.0 {
            None
        } else {
            let mut new_scene = Scene::new();
            new_scene.append(&scene, Some(Affine::scale(scale)));
            Some(new_scene)
        };
        let scene_ref = transformed_scene.as_ref().unwrap_or(&scene);

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
            .render_to_surface(device, queue, scene_ref, &surface_texture, &render_params)
            .expect("failed to render to surface");
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);
    }
}

pub(crate) fn try_init_tracing() -> Result<(), SetGlobalDefaultError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::EnvFilter;

        let default_level = if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        };
        let env_filter = EnvFilter::builder()
            .with_default_directive(default_level.into())
            .with_env_var("RUST_LOG")
            .from_env_lossy();
        let fmt_layer = tracing_subscriber::fmt::layer()
            // Display target (eg "my_crate::some_mod::submod") with logs
            .with_target(true);

        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer);
        tracing::dispatcher::set_global_default(registry.into())
    }

    // Note - tracing-wasm might not work in headless Node.js. Probably doesn't matter anyway,
    // because this is a GUI framework, so wasm targets will virtually always be browsers.
    #[cfg(target_arch = "wasm32")]
    {
        // Ignored if the panic hook is already set
        console_error_panic_hook::set_once();

        let max_level = if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };
        let config = tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(max_level)
            .build();

        tracing::subscriber::set_global_default(
            Registry::default().with(tracing_wasm::WASMLayer::new(config)),
        )
    }
}
