// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::sync::Arc;

use accesskit_winit::Adapter;
use tracing::{debug, error, info, info_span};
use vello::kurbo::Affine;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::PresentMode;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent as WinitDeviceEvent, DeviceId, WindowEvent as WinitWindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{
    AppDriver, DriverCtx, RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy,
    masonry_resize_direction_to_winit, winit_ime_to_masonry,
};
use crate::core::{TextEvent, Widget, WidgetId, WindowEvent};
use crate::peniko::Color;
use ui_events_winit::{WindowEventReducer, WindowEventTranslation};

#[derive(Debug)]
pub enum MasonryUserEvent {
    AccessKit(accesskit_winit::Event),
    // TODO: A more considered design here
    Action(crate::core::Action, WidgetId),
}

impl From<accesskit_winit::Event> for MasonryUserEvent {
    fn from(value: accesskit_winit::Event) -> Self {
        Self::AccessKit(value)
    }
}

#[expect(unnameable_types, reason = "TODO")]
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

/// The state of the Masonry application. If you run Masonry from an external Winit event loop, create a
/// `MasonryState` via [`MasonryState::new`] and forward events to it via the appropriate method (e.g.,
/// calling [`handle_window_event`](MasonryState::handle_window_event) in [`window_event`](ApplicationHandler::window_event)).
pub struct MasonryState<'a> {
    render_cx: RenderContext,
    render_root: RenderRoot,
    renderer: Option<Renderer>,
    // TODO: Winit doesn't seem to let us create these proxies from within the loop
    // The reasons for this are unclear
    proxy: EventLoopProxy,
    #[cfg(feature = "tracy")]
    frame: Option<tracing_tracy::client::Frame>,

    // Per-Window state
    // In future, this will support multiple windows
    window: WindowState<'a>,
    event_reducer: WindowEventReducer,
    background_color: Color,
}

struct MainState<'a> {
    masonry_state: MasonryState<'a>,
    app_driver: Box<dyn AppDriver>,
}

/// The type of the event loop used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoop = winit::event_loop::EventLoop<MasonryUserEvent>;
/// The type of the event loop builder used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoopBuilder = winit::event_loop::EventLoopBuilder<MasonryUserEvent>;

/// A proxy used to send events to the event loop
pub type EventLoopProxy = winit::event_loop::EventLoopProxy<MasonryUserEvent>;

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

    run_with(
        event_loop,
        window_attributes,
        root_widget,
        app_driver,
        Color::BLACK,
    )
}

pub fn run_with(
    event_loop: EventLoop,
    window: WindowAttributes,
    root_widget: impl Widget,
    app_driver: impl AppDriver + 'static,
    background_color: Color,
) -> Result<(), EventLoopError> {
    // If there is no default tracing subscriber, we set our own. If one has
    // already been set, we get an error which we swallow.
    // By now, we're about to take control of the event loop. The user is unlikely
    // to try to set their own subscriber once the event loop has started.
    let _ = crate::app::try_init_tracing();

    let mut main_state = MainState {
        masonry_state: MasonryState::new(window, &event_loop, root_widget, background_color),
        app_driver: Box::new(app_driver),
    };
    main_state
        .app_driver
        .on_start(&mut main_state.masonry_state);

    event_loop.run_app(&mut main_state)
}

impl ApplicationHandler<MasonryUserEvent> for MainState<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry_state
            .handle_resumed(event_loop, &mut *self.app_driver);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry_state.handle_suspended(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        self.masonry_state.handle_window_event(
            event_loop,
            window_id,
            event,
            self.app_driver.as_mut(),
        );
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: WinitDeviceEvent,
    ) {
        self.masonry_state.handle_device_event(
            event_loop,
            device_id,
            event,
            self.app_driver.as_mut(),
        );
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: MasonryUserEvent) {
        self.masonry_state
            .handle_user_event(event_loop, event, self.app_driver.as_mut());
    }

    // The following have empty handlers, but adding this here for future proofing. E.g., memory
    // warning is very likely to be handled for mobile and we in particular want to make sure
    // external event loops can let masonry handle these callbacks.

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_about_to_wait(event_loop);
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.masonry_state.handle_new_events(event_loop, cause);
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_memory_warning(event_loop);
    }
}

impl MasonryState<'_> {
    pub fn new(
        window: WindowAttributes,
        event_loop: &EventLoop,
        root_widget: impl Widget,
        background_color: Color,
    ) -> Self {
        let render_cx = RenderContext::new();
        // TODO: We can't know this scale factor until later?
        let scale_factor = 1.0;

        MasonryState {
            render_cx,
            render_root: RenderRoot::new(
                root_widget,
                RenderRootOptions {
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    scale_factor,
                    test_font: None,
                },
            ),
            renderer: None,
            #[cfg(feature = "tracy")]
            frame: None,
            proxy: event_loop.create_proxy(),

            window: WindowState::Uninitialized(window),
            event_reducer: WindowEventReducer::default(),
            background_color,
        }
    }

    // --- MARK: RESUMED ---
    pub fn handle_resumed(&mut self, event_loop: &ActiveEventLoop, app_driver: &mut dyn AppDriver) {
        match std::mem::replace(
            &mut self.window,
            // TODO: Is there a better default value which could be used?
            WindowState::Uninitialized(WindowAttributes::default()),
        ) {
            WindowState::Uninitialized(attributes) => {
                let visible = attributes.visible;
                let attributes = attributes.with_visible(false);

                let window = event_loop.create_window(attributes).unwrap();

                let adapter =
                    Adapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
                let window = Arc::new(window);
                // https://github.com/rust-windowing/winit/issues/2308
                #[cfg(target_os = "ios")]
                let size = window.outer_size();
                #[cfg(not(target_os = "ios"))]
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
                // Handle any signals caused by the initial layout or by the rescale
                self.handle_signals(event_loop, app_driver);
                // Render one frame before showing the window to avoid flashing
                if visible {
                    let (scene, tree_update) = self.render_root.redraw();
                    self.render(scene);
                    if let WindowState::Rendering {
                        window,
                        accesskit_adapter,
                        ..
                    } = &mut self.window
                    {
                        accesskit_adapter.update_if_active(|| tree_update);
                        window.set_visible(true);
                    };
                }
            }
            WindowState::Suspended {
                window,
                accesskit_adapter,
            } => {
                // https://github.com/rust-windowing/winit/issues/2308
                #[cfg(target_os = "ios")]
                let size = window.outer_size();
                #[cfg(not(target_os = "ios"))]
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

    // --- MARK: SUSPENDED ---
    pub fn handle_suspended(&mut self, _event_loop: &ActiveEventLoop) {
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
        // https://github.com/rust-windowing/winit/issues/2308
        #[cfg(target_os = "ios")]
        let size = window.outer_size();
        #[cfg(not(target_os = "ios"))]
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

        let dev_id = surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            antialiasing_support: AaSupport::area_only(),
            ..Default::default()
        };
        let render_params = RenderParams {
            base_color: self.background_color,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };

        let _render_span = tracing::info_span!("Rendering using Vello").entered();
        self.renderer
            .get_or_insert_with(|| {
                #[cfg_attr(not(feature = "tracy"), expect(unused_mut, reason = "cfg"))]
                let mut renderer = Renderer::new(device, renderer_options).unwrap();
                #[cfg(feature = "tracy")]
                {
                    let new_profiler = wgpu_profiler::GpuProfiler::new_with_tracy_client(
                        wgpu_profiler::GpuProfilerSettings::default(),
                        // We don't have access to the adapter until we get  https://github.com/linebender/vello/pull/634
                        // Luckily, this `backend` is only used for visual display in the profiling, so we can just guess here
                        wgpu::Backend::Vulkan,
                        device,
                        queue,
                    )
                    .unwrap_or(renderer.profiler);
                    renderer.profiler = new_profiler;
                }
                renderer
            })
            .render_to_texture(
                device,
                queue,
                scene_ref,
                &surface.target_view,
                &render_params,
            )
            .expect("failed to render to surface");

        let Ok(surface_texture) = surface.surface.get_current_texture() else {
            tracing::error!("failed to acquire next swapchain texture");
            return;
        };

        // Copy the new surface content to the surface.
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
            device.poll(wgpu::Maintain::Wait);
        }

        #[cfg(feature = "tracy")]
        drop(self.frame.take());
    }

    // --- MARK: WINDOW_EVENT ---
    pub fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: WindowId,
        event: WinitWindowEvent,
        app_driver: &mut dyn AppDriver,
    ) {
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
        #[cfg(feature = "tracy")]
        if self.frame.is_none() {
            self.frame = Some(tracing_tracy::client::non_continuous_frame!("Masonry"));
        }
        accesskit_adapter.process_event(window, &event);

        if !matches!(
            event,
            WinitWindowEvent::KeyboardInput {
                is_synthetic: true,
                ..
            }
        ) {
            if let Some(wet) = self.event_reducer.reduce(&event) {
                match wet {
                    WindowEventTranslation::Keyboard(k) => {
                        self.render_root.handle_text_event(TextEvent::Keyboard(k));
                    }
                    WindowEventTranslation::Pointer(p) => {
                        self.render_root.handle_pointer_event(p);
                    }
                }
            }
        }

        match event {
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.render_root
                    .handle_window_event(WindowEvent::Rescale(scale_factor));
            }
            WinitWindowEvent::RedrawRequested => {
                let _span = info_span!("redraw");
                self.render_root.handle_window_event(WindowEvent::AnimFrame);
                let (scene, tree_update) = self.render_root.redraw();
                self.render(scene);
                let WindowState::Rendering {
                    accesskit_adapter, ..
                } = &mut self.window
                else {
                    error!("Suspended inside event");
                    return;
                };
                accesskit_adapter.update_if_active(|| tree_update);
            }
            WinitWindowEvent::CloseRequested => {
                // HACK: When we exit, on some systems (known to happen with Wayland on KDE),
                // the IME state gets preserved until the app next opens. We work around this by force-deleting
                // the IME state just before exiting.
                window.set_ime_allowed(false);
                event_loop.exit();
            }
            WinitWindowEvent::Resized(size) => {
                self.render_root
                    .handle_window_event(WindowEvent::Resize(size));
            }
            WinitWindowEvent::Ime(ime) => {
                let ime = winit_ime_to_masonry(ime);
                self.render_root.handle_text_event(TextEvent::Ime(ime));
            }
            WinitWindowEvent::Focused(new_focus) => {
                self.render_root
                    .handle_text_event(TextEvent::WindowFocusChange(new_focus));
            }
            _ => (),
        }

        self.handle_signals(event_loop, app_driver);
    }

    // --- MARK: DEVICE_EVENT ---
    pub fn handle_device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: DeviceId,
        _: WinitDeviceEvent,
        _: &mut dyn AppDriver,
    ) {
    }

    // --- MARK: USER_EVENT ---
    pub fn handle_user_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: MasonryUserEvent,
        app_driver: &mut dyn AppDriver,
    ) {
        match event {
            MasonryUserEvent::AccessKit(event) => {
                match event.window_event {
                    // Note that this event can be called at any time, even multiple times if
                    // the user restarts their screen reader.
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        self.render_root
                            .handle_window_event(WindowEvent::RebuildAccessTree);
                    }
                    accesskit_winit::WindowEvent::ActionRequested(action_request) => {
                        self.render_root.handle_access_event(action_request);
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
            // TODO - Not sure what the use-case for this is.
            MasonryUserEvent::Action(action, widget) => self
                .render_root
                .emit_signal(RenderRootSignal::Action(action, widget)),
        }

        self.handle_signals(event_loop, app_driver);
    }

    // --- MARK: EMPTY WINIT HANDLERS ---
    pub fn handle_about_to_wait(&mut self, _: &ActiveEventLoop) {}

    pub fn handle_new_events(&mut self, _: &ActiveEventLoop, _: winit::event::StartCause) {}

    pub fn handle_exiting(&mut self, _: &ActiveEventLoop) {}

    pub fn handle_memory_warning(&mut self, _: &ActiveEventLoop) {}

    // --- MARK: SIGNALS ---
    fn handle_signals(&mut self, event_loop: &ActiveEventLoop, app_driver: &mut dyn AppDriver) {
        let WindowState::Rendering { window, .. } = &mut self.window else {
            tracing::warn!("Tried to handle a signal whilst suspended or before window created");
            return;
        };

        let mut needs_redraw = false;
        while let Some(signal) = self.render_root.pop_signal() {
            match signal {
                RenderRootSignal::Action(action, widget_id) => {
                    let mut driver_ctx = DriverCtx {
                        render_root: &mut self.render_root,
                    };
                    debug!("Action {:?} on widget {:?}", action, widget_id);
                    app_driver.on_action(&mut driver_ctx, widget_id, action);
                }
                RenderRootSignal::StartIme => {
                    window.set_ime_allowed(true);
                }
                RenderRootSignal::EndIme => {
                    window.set_ime_allowed(false);
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    window.set_ime_cursor_area(position, size);
                }
                RenderRootSignal::RequestRedraw => {
                    needs_redraw = true;
                }
                RenderRootSignal::RequestAnimFrame => {
                    // TODO
                    needs_redraw = true;
                }
                RenderRootSignal::TakeFocus => {
                    window.focus_window();
                }
                RenderRootSignal::SetCursor(cursor) => {
                    window.set_cursor(cursor);
                }
                RenderRootSignal::SetSize(size) => {
                    // TODO - Handle return value?
                    let _ = window.request_inner_size(size);
                }
                RenderRootSignal::SetTitle(title) => {
                    window.set_title(&title);
                }
                RenderRootSignal::DragWindow => {
                    // TODO - Handle return value?
                    let _ = window.drag_window();
                }
                RenderRootSignal::DragResizeWindow(direction) => {
                    // TODO - Handle return value?
                    let direction = masonry_resize_direction_to_winit(direction);
                    let _ = window.drag_resize_window(direction);
                }
                RenderRootSignal::ToggleMaximized => {
                    window.set_maximized(!window.is_maximized());
                }
                RenderRootSignal::Minimize => {
                    window.set_minimized(true);
                }
                RenderRootSignal::Exit => {
                    event_loop.exit();
                }
                RenderRootSignal::ShowWindowMenu(position) => {
                    window.show_window_menu(position);
                }
                RenderRootSignal::WidgetSelectedInInspector(widget_id) => {
                    let Some(widget) = self.render_root.get_widget(widget_id) else {
                        return;
                    };
                    let widget_name = widget.short_type_name();
                    let display_name = if let Some(debug_text) = widget.get_debug_text() {
                        format!("{widget_name}<{debug_text}>")
                    } else {
                        widget_name.into()
                    };
                    info!("Widget selected in inspector: {widget_id} - {display_name}");
                }
            }
        }

        // If we're processing a lot of actions, we may have a lot of pending redraws.
        // We batch them up to avoid redundant requests.
        if needs_redraw {
            window.request_redraw();
        }
    }

    pub fn get_window_state(&self) -> &WindowState<'_> {
        &self.window
    }

    pub fn get_root(&mut self) -> &mut RenderRoot {
        &mut self.render_root
    }

    pub fn set_present_mode(&mut self, present_mode: wgpu::PresentMode) {
        if let WindowState::Rendering { surface, .. } = &mut self.window {
            self.render_cx.set_present_mode(surface, present_mode);
        }
    }
}
