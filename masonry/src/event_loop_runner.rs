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
use winit::error::EventLoopError;
use winit::event::{MouseButton as WinitMouseButton, WindowEvent as WinitWindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app_driver::{AppDriver, DriverCtx};
use crate::dpi::LogicalPosition;
use crate::event::{PointerButton, PointerState, WindowEvent};
use crate::render_root::{self, RenderRoot, WindowSizePolicy};
use crate::{PointerEvent, TextEvent, Widget};

impl From<WinitMouseButton> for PointerButton {
    fn from(button: WinitMouseButton) -> Self {
        match button {
            WinitMouseButton::Left => PointerButton::Primary,
            WinitMouseButton::Right => PointerButton::Secondary,
            WinitMouseButton::Middle => PointerButton::Auxiliary,
            WinitMouseButton::Back => PointerButton::X1,
            WinitMouseButton::Forward => PointerButton::X2,
            WinitMouseButton::Other(other) => {
                warn!("Got winit MouseButton::Other({other}) which is not yet fully supported.");
                PointerButton::Other
            }
        }
    }
}

pub enum WindowState<'a> {
    Uninitialized(WindowAttributes),
    Rendering {
        window: Arc<Window>,
        surface: RenderSurface<'a>,
        accesskit_adapter: Adapter,
    },
    Suspended {
        window: Arc<Window>,
        accesskit_adapter: Adapter,
    },
}

struct MainState<'a> {
    render_cx: RenderContext,
    render_root: RenderRoot,
    pointer_state: PointerState,
    app_driver: Box<dyn AppDriver>,
    renderer: Option<Renderer>,
    // TODO: Winit doesn't seem to let us create these proxies from within the loop
    // The reasons for this are unclear
    proxy: EventLoopProxy<accesskit_winit::Event>,

    // Per-Window state
    // In future, this will support multiple windows
    window: WindowState<'a>,
}

/// The type of the event loop used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoop = winit::event_loop::EventLoop<accesskit_winit::Event>;
/// The type of the event loop builder used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoopBuilder = winit::event_loop::EventLoopBuilder<accesskit_winit::Event>;

// --- MARK: RUN ---
pub fn run(
    // Clearly, this API needs to be refactored, so we don't mind forcing this to be passed in here directly
    // This is passed in mostly to allow configuring the Android app
    mut loop_builder: EventLoopBuilder,
    // In future, we intend to support multiple windows. At the moment though, we only support one
    window_attributes: WindowAttributes,
    root_widget: impl Widget,
    app_driver: impl AppDriver + 'static,
) -> Result<(), EventLoopError> {
    let event_loop = loop_builder.build()?;

    run_with(window_attributes, event_loop, root_widget, app_driver)
}

pub fn run_with(
    window: WindowAttributes,
    event_loop: EventLoop,
    root_widget: impl Widget,
    app_driver: impl AppDriver + 'static,
) -> Result<(), EventLoopError> {
    let render_cx = RenderContext::new();
    // TODO: We can't know this scale factor until later?
    let scale_factor = 1.0;
    let mut main_state = MainState {
        render_cx,
        render_root: RenderRoot::new(root_widget, WindowSizePolicy::User, scale_factor),
        renderer: None,
        pointer_state: PointerState::empty(),
        app_driver: Box::new(app_driver),
        proxy: event_loop.create_proxy(),

        window: WindowState::Uninitialized(window),
    };

    // If there is no default tracing subscriber, we set our own. If one has
    // already been set, we get an error which we swallow.
    // By now, we're about to take control of the event loop. The user is unlikely
    // to try to set their own subscriber once the event loop has started.
    let _ = try_init_tracing();

    event_loop.run_app(&mut main_state)
}

impl ApplicationHandler<accesskit_winit::Event> for MainState<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match std::mem::replace(
            &mut self.window,
            // TODO: Is there a better default value which could be used?
            WindowState::Uninitialized(WindowAttributes::default()),
        ) {
            WindowState::Uninitialized(attributes) => {
                let visible = attributes.visible;
                let attributes = attributes.with_visible(false);

                let window = event_loop.create_window(attributes).unwrap();

                let adapter = Adapter::with_event_loop_proxy(&window, self.proxy.clone());
                window.set_visible(visible);
                let window = Arc::new(window);
                let size = window.inner_size();
                let surface = pollster::block_on(self.render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    PresentMode::AutoVsync,
                ))
                .unwrap();
                let scale_factor = window.scale_factor();
                self.window = WindowState::Rendering {
                    window,
                    surface,
                    accesskit_adapter: adapter,
                };
                self.render_root
                    .handle_window_event(WindowEvent::Rescale(scale_factor));
            }
            WindowState::Suspended {
                window,
                accesskit_adapter,
            } => {
                let size = window.inner_size();
                let surface = pollster::block_on(self.render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    PresentMode::AutoVsync,
                ))
                .unwrap();
                self.window = WindowState::Rendering {
                    window,
                    surface,
                    accesskit_adapter,
                }
            }
            _ => {
                // We have received a redundant resumed event. That's allowed by winit
            }
        }
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        match std::mem::replace(
            &mut self.window,
            // TODO: Is there a better default value which could be used?
            WindowState::Uninitialized(WindowAttributes::default()),
        ) {
            WindowState::Rendering {
                window,
                surface,
                accesskit_adapter,
            } => {
                drop(surface);
                self.window = WindowState::Suspended {
                    window,
                    accesskit_adapter,
                };
            }
            _ => {
                // We have received a redundant resumed event. That's allowed by winit
            }
        }
    }

    // --- MARK: WINDOW_EVENT ---
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WinitWindowEvent) {
        let WindowState::Rendering {
            window,
            accesskit_adapter,
            ..
        } = &mut self.window
        else {
            tracing::warn!(
                ?event,
                "Got window event whilst suspended or before window created"
            );
            return;
        };
        accesskit_adapter.process_event(window, &event);

        match event {
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.render_root
                    .handle_window_event(WindowEvent::Rescale(scale_factor));
            }
            WinitWindowEvent::RedrawRequested => {
                let (scene, tree_update) = self.render_root.redraw();
                self.render(scene);
                let WindowState::Rendering {
                    accesskit_adapter, ..
                } = &mut self.window
                else {
                    debug_panic!("Suspended inside event");
                    return;
                };
                accesskit_adapter.update_if_active(|| tree_update);
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
                is_synthetic: false, // TODO: Introduce an escape hatch for synthetic keys
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
            WinitWindowEvent::CursorEntered { .. } => {
                self.render_root
                    .handle_pointer_event(PointerEvent::PointerEnter(self.pointer_state.clone()));
            }
            WinitWindowEvent::CursorMoved { position, .. } => {
                self.pointer_state.physical_position = position;
                self.pointer_state.position = position.to_logical(window.scale_factor());
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
                            button.into(),
                            self.pointer_state.clone(),
                        ));
                }
                winit::event::ElementState::Released => {
                    self.render_root
                        .handle_pointer_event(PointerEvent::PointerUp(
                            button.into(),
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
                        delta.to_logical(window.scale_factor())
                    }
                };
                self.render_root
                    .handle_pointer_event(PointerEvent::MouseWheel(
                        delta,
                        self.pointer_state.clone(),
                    ));
            }
            WinitWindowEvent::Touch(winit::event::Touch {
                location, phase, ..
            }) => {
                // FIXME: This is naïve and should be refined for actual use.
                //        It will also interact with gesture discrimination.
                self.pointer_state.physical_position = location;
                self.pointer_state.position = location.to_logical(window.scale_factor());
                match phase {
                    winit::event::TouchPhase::Started => {
                        self.render_root
                            .handle_pointer_event(PointerEvent::PointerMove(
                                self.pointer_state.clone(),
                            ));
                        self.render_root
                            .handle_pointer_event(PointerEvent::PointerDown(
                                PointerButton::Primary,
                                self.pointer_state.clone(),
                            ));
                    }
                    winit::event::TouchPhase::Ended => {
                        self.render_root
                            .handle_pointer_event(PointerEvent::PointerUp(
                                PointerButton::Primary,
                                self.pointer_state.clone(),
                            ));
                    }
                    winit::event::TouchPhase::Moved => {
                        self.render_root
                            .handle_pointer_event(PointerEvent::PointerMove(
                                self.pointer_state.clone(),
                            ));
                    }
                    winit::event::TouchPhase::Cancelled => {
                        self.render_root
                            .handle_pointer_event(PointerEvent::PointerLeave(
                                self.pointer_state.clone(),
                            ));
                    }
                }
            }
            _ => (),
        }

        self.handle_signals(event_loop);
    }

    // --- MARK: USER_EVENT ---
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: accesskit_winit::Event) {
        match event.window_event {
            // Note that this event can be called at any time, even multiple times if
            // the user restarts their screen reader.
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                self.render_root
                    .handle_window_event(WindowEvent::RebuildAccessTree);
            }
            accesskit_winit::WindowEvent::ActionRequested(action_request) => {
                self.render_root.root_on_access_event(action_request);
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }

        self.handle_signals(event_loop);
    }
}

impl MainState<'_> {
    // --- MARK: RENDER ---
    fn render(&mut self, scene: Scene) {
        let WindowState::Rendering {
            window, surface, ..
        } = &mut self.window
        else {
            tracing::warn!("Tried to render whilst suspended or before window created");
            return;
        };
        let scale_factor = window.scale_factor();
        let size = window.inner_size();
        let width = size.width;
        let height = size.height;

        if surface.config.width != width || surface.config.height != height {
            self.render_cx.resize_surface(surface, width, height);
        }

        let transformed_scene = if scale_factor == 1.0 {
            None
        } else {
            let mut new_scene = Scene::new();
            new_scene.append(&scene, Some(Affine::scale(scale_factor)));
            Some(new_scene)
        };
        let scene_ref = transformed_scene.as_ref().unwrap_or(&scene);

        let Ok(surface_texture) = surface.surface.get_current_texture() else {
            warn!("failed to acquire next swapchain texture");
            return;
        };
        let dev_id = surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(surface.format),
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

    // --- MARK: SIGNALS ---
    fn handle_signals(&mut self, _event_loop: &ActiveEventLoop) {
        let WindowState::Rendering { window, .. } = &mut self.window else {
            tracing::warn!("Tried to handle a signal whilst suspended or before window created");
            return;
        };
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
                render_root::RenderRootSignal::StartIme => {
                    window.set_ime_allowed(true);
                }
                render_root::RenderRootSignal::EndIme => {
                    window.set_ime_allowed(false);
                }
                render_root::RenderRootSignal::ImeMoved(position, size) => {
                    window.set_ime_cursor_area(position, size);
                }
                render_root::RenderRootSignal::RequestRedraw => {
                    window.request_redraw();
                }
                render_root::RenderRootSignal::RequestAnimFrame => {
                    // TODO
                    window.request_redraw();
                }
                render_root::RenderRootSignal::SpawnWorker(_worker_fn) => {
                    // TODO
                }
                render_root::RenderRootSignal::TakeFocus => {
                    window.focus_window();
                }
                render_root::RenderRootSignal::SetCursor(cursor) => {
                    window.set_cursor(cursor);
                }
                render_root::RenderRootSignal::SetSize(size) => {
                    // TODO - Handle return value?
                    let _ = window.request_inner_size(size);
                }
                render_root::RenderRootSignal::SetTitle(title) => {
                    window.set_title(&title);
                }
            }
        }
    }
}

// --- MARK: TRACING ---
pub(crate) fn try_init_tracing() -> Result<(), SetGlobalDefaultError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use time::macros::format_description;
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::fmt::time::UtcTime;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::EnvFilter;

        // Default level is DEBUG in --dev, INFO in --release
        // DEBUG should print a few logs per low-density event.
        // INFO should only print logs for noteworthy things.
        let default_level = if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        };
        // Use EnvFilter to allow the user to override the log level without recompiling.
        // TODO - Print error message if the env var is incorrectly formatted.
        let env_filter = EnvFilter::builder()
            .with_default_directive(default_level.into())
            .with_env_var("RUST_LOG")
            .from_env_lossy();
        // This format is more concise than even the 'Compact' default:
        // - We print the time without the date (GUI apps usually run for very short periods).
        // - We print the time with milliseconds precision.
        // - We skip the target. In app code, the target is almost always visual noise. By
        //   default, it only gives you the module a log was defined in. This is rarely useful;
        //   the log message is much more helpful for finding a log's location.
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_timer(UtcTime::new(format_description!(
                // We append a `Z` here to indicate clearly that this is a UTC time
                "[hour repr:24]:[minute]:[second].[subsecond digits:3]Z"
            )))
            .with_target(false);

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
