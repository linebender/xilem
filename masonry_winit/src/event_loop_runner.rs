// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, mpsc};

use accesskit_winit::Adapter;
use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::{DefaultProperties, TextEvent, Widget, WidgetId, WidgetPod, WindowEvent};
use masonry::kurbo::Affine;
use masonry::peniko::Color;
use masonry::theme::default_property_set;
use masonry::util::Instant;
use masonry::vello::util::{RenderContext, RenderSurface};
use masonry::vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use tracing::{debug, error, info, info_span};
use ui_events_winit::{WindowEventReducer, WindowEventTranslation};
use wgpu::PresentMode;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent as WinitDeviceEvent, DeviceId, WindowEvent as WinitWindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window as WindowHandle, WindowAttributes, WindowId as HandleId};

use crate::app::{AppDriver, DriverCtx, masonry_resize_direction_to_winit, winit_ime_to_masonry};
use crate::app_driver::WindowId;

#[derive(Debug)]
pub enum MasonryUserEvent {
    AccessKit(HandleId, accesskit_winit::WindowEvent),
    // TODO: A more considered design here
    Action(WindowId, masonry::core::Action, WidgetId),
}

impl From<accesskit_winit::Event> for MasonryUserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::AccessKit(event.window_id, event.window_event)
    }
}

#[expect(unnameable_types, reason = "TODO")]
#[allow(
    clippy::large_enum_variant,
    reason = "we don't have that many instances of it"
)]
pub enum WindowState {
    Uninitialized(WindowAttributes),
    Rendering {
        handle: Arc<WindowHandle>,
        accesskit_adapter: Adapter,
    },
    Suspended {
        handle: Arc<WindowHandle>,
        accesskit_adapter: Adapter,
    },
}

impl Debug for WindowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uninitialized(attrs) => f.debug_tuple("Uninitialized").field(attrs).finish(),
            Self::Rendering { .. } => f.debug_struct("Rendering").finish_non_exhaustive(),
            Self::Suspended { .. } => f.debug_struct("Suspended").finish_non_exhaustive(),
        }
    }
}

/// Per-Window state
pub(crate) struct Window {
    id: WindowId,
    pub(crate) state: WindowState,
    event_reducer: WindowEventReducer,
    pub(crate) render_root: RenderRoot,
}

impl Window {
    pub(crate) fn new(
        window_id: WindowId,
        root_widget: WidgetPod<dyn Widget>,
        attributes: WindowAttributes,
        signal_sender: Arc<Mutex<Sender<(WindowId, RenderRootSignal)>>>,
        default_properties: Arc<DefaultProperties>,
    ) -> Self {
        // TODO: We can't know this scale factor until later?
        let scale_factor = 1.0;

        Self {
            id: window_id,
            state: WindowState::Uninitialized(attributes),
            event_reducer: WindowEventReducer::default(),
            render_root: RenderRoot::new(
                root_widget,
                move |signal| {
                    signal_sender
                        .lock()
                        .unwrap()
                        .send((window_id, signal))
                        .unwrap();
                },
                RenderRootOptions {
                    default_properties,
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    scale_factor,
                    test_font: None,
                },
            ),
        }
    }
}

/// The state of the Masonry application. If you run Masonry from an external Winit event loop, create a
/// `MasonryState` via [`MasonryState::new`] and forward events to it via the appropriate method (e.g.,
/// calling [`handle_window_event`](MasonryState::handle_window_event) in [`window_event`](ApplicationHandler::window_event)).
pub struct MasonryState<'a> {
    render_cx: RenderContext,
    renderer: Option<Renderer>,
    // TODO: Winit doesn't seem to let us create these proxies from within the loop
    // The reasons for this are unclear
    event_loop_proxy: EventLoopProxy,
    #[cfg(feature = "tracy")]
    frame: Option<tracing_tracy::client::Frame>,

    window_id_to_handle_id: HashMap<WindowId, HandleId>,
    windows: HashMap<HandleId, Window>,
    surfaces: HashMap<HandleId, RenderSurface<'a>>,

    // Is `Some` if the most recently displayed frame was an animation frame.
    last_anim: Option<Instant>,
    signal_receiver: mpsc::Receiver<(WindowId, RenderRootSignal)>,

    signal_sender: Arc<Mutex<Sender<(WindowId, RenderRootSignal)>>>,
    default_properties: Arc<DefaultProperties>,
    pub(crate) exit: bool,
    initial_windows: Vec<(WindowId, WindowAttributes, WidgetPod<dyn Widget>)>,
    need_first_frame: Vec<HandleId>,
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

// --- MARK: RUN
pub fn run(
    // Clearly, this API needs to be refactored, so we don't mind forcing this to be passed in here directly
    // This is passed in mostly to allow configuring the Android app
    mut loop_builder: EventLoopBuilder,
    windows: Vec<(WindowId, WindowAttributes, WidgetPod<dyn Widget>)>,
    app_driver: impl AppDriver + 'static,
) -> Result<(), EventLoopError> {
    let event_loop = loop_builder.build()?;

    run_with(event_loop, windows, app_driver, default_property_set())
}

pub fn run_with(
    event_loop: EventLoop,
    windows: Vec<(WindowId, WindowAttributes, WidgetPod<dyn Widget>)>,
    app_driver: impl AppDriver + 'static,
    default_properties: DefaultProperties,
) -> Result<(), EventLoopError> {
    // If there is no default tracing subscriber, we set our own. If one has
    // already been set, we get an error which we swallow.
    // By now, we're about to take control of the event loop. The user is unlikely
    // to try to set their own subscriber once the event loop has started.
    let _ = masonry::app::try_init_tracing();

    let mut main_state = MainState {
        masonry_state: MasonryState::new(event_loop.create_proxy(), windows, default_properties),
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
        handle_id: HandleId,
        event: WinitWindowEvent,
    ) {
        self.masonry_state.handle_window_event(
            event_loop,
            handle_id,
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
        event_loop_proxy: EventLoopProxy,
        initial_windows: Vec<(WindowId, WindowAttributes, WidgetPod<dyn Widget>)>,
        default_properties: DefaultProperties,
    ) -> Self {
        let render_cx = RenderContext::new();

        let (signal_sender, signal_receiver) =
            std::sync::mpsc::channel::<(WindowId, RenderRootSignal)>();
        let signal_sender = Arc::new(Mutex::new(signal_sender));

        MasonryState {
            render_cx,
            renderer: None,
            event_loop_proxy,
            #[cfg(feature = "tracy")]
            frame: None,
            signal_receiver,

            last_anim: None,
            window_id_to_handle_id: HashMap::new(),
            windows: HashMap::new(),
            surfaces: HashMap::new(),

            signal_sender,
            default_properties: Arc::new(default_properties),
            exit: false,
            initial_windows,
            need_first_frame: Vec::new(),
        }
    }

    // --- MARK: RESUMED
    pub fn handle_resumed(&mut self, event_loop: &ActiveEventLoop, app_driver: &mut dyn AppDriver) {
        if !self.initial_windows.is_empty() {
            for (id, attrs, widget) in std::mem::take(&mut self.initial_windows) {
                self.create_window(event_loop, id, attrs, widget);
            }
        } else {
            for mut window in std::mem::take(&mut self.windows).into_values() {
                match std::mem::replace(
                    &mut window.state,
                    // TODO: Is there a better default value which could be used?
                    WindowState::Uninitialized(WindowAttributes::default()),
                ) {
                    WindowState::Uninitialized(attributes) => {
                        self.create_window_inner(event_loop, attributes, window);
                    }
                    WindowState::Suspended {
                        handle,
                        accesskit_adapter,
                    } => {
                        window.state = WindowState::Rendering {
                            handle,
                            accesskit_adapter,
                        };
                    }
                    _ => {
                        // We have received a redundant resumed event. That's allowed by winit
                    }
                }
            }
        }

        self.handle_signals(event_loop, app_driver);
    }

    pub(crate) fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        attributes: WindowAttributes,
        root_widget: WidgetPod<dyn Widget>,
    ) {
        let window = Window::new(
            window_id,
            root_widget,
            attributes.clone(),
            self.signal_sender.clone(),
            self.default_properties.clone(),
        );
        self.create_window_inner(event_loop, attributes, window);
    }

    pub(crate) fn create_window_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
        mut window: Window,
    ) {
        if self.window_id_to_handle_id.contains_key(&window.id) {
            panic!(
                "attempted to create a window with id {:?} but a window with that id already exists",
                window.id
            );
        }

        let visible = attributes.visible;
        // We always create the window as invisible so that we can
        // render the first frame before showing it to avoid flashing.
        let handle = event_loop
            .create_window(attributes.with_visible(false))
            .unwrap();
        if visible {
            // We defer the rendering of the first frame to the handle_signals method because
            // we want to handle any signals caused by the initial layout or rescale before we render.
            self.need_first_frame.push(handle.id());
        }

        let adapter =
            Adapter::with_event_loop_proxy(event_loop, &handle, self.event_loop_proxy.clone());
        let handle = Arc::new(handle);
        // https://github.com/rust-windowing/winit/issues/2308
        #[cfg(target_os = "ios")]
        let size = handle.outer_size();
        #[cfg(not(target_os = "ios"))]
        let size = handle.inner_size();
        let surface = pollster::block_on(self.render_cx.create_surface(
            handle.clone(),
            size.width,
            size.height,
            PresentMode::AutoVsync,
        ))
        .unwrap();
        let scale_factor = handle.scale_factor();
        let handle_id = handle.id();
        window.state = WindowState::Rendering {
            handle,
            accesskit_adapter: adapter,
        };
        window
            .render_root
            .handle_window_event(WindowEvent::Rescale(scale_factor));

        tracing::debug!(window_id = window.id.trace(), handle=?handle_id, "creating window");
        self.window_id_to_handle_id.insert(window.id, handle_id);
        self.windows.insert(handle_id, window);
        self.surfaces.insert(handle_id, surface);
    }

    pub fn close_window(&mut self, window_id: WindowId) {
        tracing::debug!(window_id = window_id.trace(), "closing window");
        let window_id = self
            .window_id_to_handle_id
            .remove(&window_id)
            .unwrap_or_else(|| panic!("could not found find window for id {window_id:?}"));
        self.surfaces.remove(&window_id);
        let window = self.windows.remove(&window_id).unwrap();

        // HACK: When we exit, on some systems (known to happen with Wayland on KDE),
        // the IME state gets preserved until the app next opens. We work around this by force-deleting
        // the IME state just before exiting.
        if let WindowState::Rendering { handle, .. } = window.state {
            handle.set_ime_allowed(false);
        }
    }

    // --- MARK: SUSPENDED
    pub fn handle_suspended(&mut self, _event_loop: &ActiveEventLoop) {
        for window in self.windows.values_mut() {
            match std::mem::replace(
                &mut window.state,
                // TODO: Is there a better default value which could be used?
                WindowState::Uninitialized(WindowAttributes::default()),
            ) {
                WindowState::Rendering {
                    handle,
                    accesskit_adapter,
                } => {
                    window.state = WindowState::Suspended {
                        handle,
                        accesskit_adapter,
                    };
                }
                _ => {
                    // We have received a redundant resumed event. That's allowed by winit
                }
            }
        }
        self.surfaces.clear();
    }

    // --- MARK: RENDER ---
    fn render(
        surface: &mut RenderSurface<'_>,
        window: &mut Window,
        scene: Scene,
        render_cx: &RenderContext,
        renderer: &mut Option<Renderer>,
    ) {
        let WindowState::Rendering { handle, .. } = &mut window.state else {
            tracing::warn!("Tried to render whilst suspended or before window created");
            return;
        };
        let scale_factor = handle.scale_factor();
        // https://github.com/rust-windowing/winit/issues/2308
        #[cfg(target_os = "ios")]
        let size = handle.outer_size();
        #[cfg(not(target_os = "ios"))]
        let size = handle.inner_size();
        let width = size.width;
        let height = size.height;

        if surface.config.width != width || surface.config.height != height {
            render_cx.resize_surface(surface, width, height);
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
        let device = &render_cx.devices[dev_id].device;
        let queue = &render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            antialiasing_support: AaSupport::area_only(),
            ..Default::default()
        };
        let render_params = RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };

        let _render_span = tracing::info_span!("Rendering using Vello").entered();
        renderer
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
        handle.pre_present_notify();
        surface_texture.present();
        {
            let _render_poll_span =
                tracing::info_span!("Waiting for GPU to finish rendering").entered();
            device.poll(wgpu::Maintain::Wait);
        }
    }

    // --- MARK: WINDOW_EVENT
    pub fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        handle_id: HandleId,
        event: WinitWindowEvent,
        app_driver: &mut dyn AppDriver,
    ) {
        let Some(window) = self.windows.get_mut(&handle_id) else {
            tracing::warn!(
                ?event,
                "Got window event for unknown window {:?}",
                handle_id
            );
            return;
        };
        let _span = info_span!("window_event", window_id = window.id.trace()).entered();
        let WindowState::Rendering {
            handle,
            accesskit_adapter,
            ..
        } = &mut window.state
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
        accesskit_adapter.process_event(handle, &event);

        if !matches!(
            event,
            WinitWindowEvent::KeyboardInput {
                is_synthetic: true,
                ..
            }
        ) {
            if let Some(wet) = window.event_reducer.reduce(&event) {
                match wet {
                    WindowEventTranslation::Keyboard(k) => {
                        window.render_root.handle_text_event(TextEvent::Keyboard(k));
                    }
                    WindowEventTranslation::Pointer(p) => {
                        window.render_root.handle_pointer_event(p);
                    }
                }
            }
        }

        match event {
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                window
                    .render_root
                    .handle_window_event(WindowEvent::Rescale(scale_factor));
            }
            WinitWindowEvent::RedrawRequested => {
                let _span = info_span!("redraw");

                let now = Instant::now();
                // TODO: this calculation uses wall-clock time of the paint call, which
                // potentially has jitter.
                //
                // See https://github.com/linebender/druid/issues/85 for discussion.
                let last = self.last_anim.take();
                let elapsed = last.map(|t| now.duration_since(t)).unwrap_or_default();

                window
                    .render_root
                    .handle_window_event(WindowEvent::AnimFrame(elapsed));

                // If this animation will continue, store the time.
                // If a new animation starts, then it will have zero reported elapsed time.
                let animation_continues = window.render_root.needs_anim();
                self.last_anim = animation_continues.then_some(now);

                let (scene, tree_update) = window.render_root.redraw();
                Self::render(
                    self.surfaces.get_mut(&handle_id).unwrap(),
                    window,
                    scene,
                    &self.render_cx,
                    &mut self.renderer,
                );
                #[cfg(feature = "tracy")]
                drop(self.frame.take());
                let WindowState::Rendering {
                    accesskit_adapter, ..
                } = &mut window.state
                else {
                    error!("Suspended inside event");
                    return;
                };
                accesskit_adapter.update_if_active(|| tree_update);
            }
            WinitWindowEvent::CloseRequested => {
                app_driver.on_close_requested(window.id, &mut DriverCtx::new(self, event_loop));
            }
            WinitWindowEvent::Resized(size) => {
                window
                    .render_root
                    .handle_window_event(WindowEvent::Resize(size));
            }
            WinitWindowEvent::Ime(ime) => {
                let ime = winit_ime_to_masonry(ime);
                window.render_root.handle_text_event(TextEvent::Ime(ime));
            }
            WinitWindowEvent::Focused(new_focus) => {
                window
                    .render_root
                    .handle_text_event(TextEvent::WindowFocusChange(new_focus));
            }
            _ => (),
        }

        self.handle_signals(event_loop, app_driver);
        if self.exit {
            event_loop.exit();
        }
    }

    // --- MARK: DEVICE_EVENT
    pub fn handle_device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: DeviceId,
        _: WinitDeviceEvent,
        _: &mut dyn AppDriver,
    ) {
    }

    // --- MARK: USER_EVENT
    pub fn handle_user_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: MasonryUserEvent,
        app_driver: &mut dyn AppDriver,
    ) {
        let state = match &event {
            MasonryUserEvent::AccessKit(handle_id, ..) => {
                let Some(state) = self.windows.get_mut(handle_id) else {
                    tracing::warn!(handle = ?handle_id, "Got accesskit user event for unknown window");
                    return;
                };
                state
            }
            MasonryUserEvent::Action(window_id, ..) => {
                let Some(window_id) = self.window_id_to_handle_id.get(window_id) else {
                    tracing::warn!(id = ?window_id, "Got action user event for unknown window");
                    return;
                };
                self.windows.get_mut(window_id).unwrap()
            }
        };
        match event {
            MasonryUserEvent::AccessKit(_, event) => {
                match event {
                    // Note that this event can be called at any time, even multiple times if
                    // the user restarts their screen reader.
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        state
                            .render_root
                            .handle_window_event(WindowEvent::RebuildAccessTree);
                    }
                    accesskit_winit::WindowEvent::ActionRequested(action_request) => {
                        state.render_root.handle_access_event(action_request);
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
            // TODO - Not sure what the use-case for this is.
            MasonryUserEvent::Action(_, action, widget) => state
                .render_root
                .emit_signal(RenderRootSignal::Action(action, widget)),
        }

        self.handle_signals(event_loop, app_driver);
    }

    // --- MARK: EMPTY WINIT HANDLERS
    pub fn handle_about_to_wait(&mut self, _: &ActiveEventLoop) {}

    pub fn handle_new_events(&mut self, _: &ActiveEventLoop, _: winit::event::StartCause) {}

    pub fn handle_exiting(&mut self, _: &ActiveEventLoop) {}

    pub fn handle_memory_warning(&mut self, _: &ActiveEventLoop) {}

    // --- MARK: SIGNALS
    fn handle_signals(&mut self, event_loop: &ActiveEventLoop, app_driver: &mut dyn AppDriver) {
        let mut need_redraw = HashSet::<HandleId>::new();

        loop {
            let Some((window_id, signal)) = self.signal_receiver.try_iter().next() else {
                break;
            };

            let Some(handle_id) = self.window_id_to_handle_id.get(&window_id) else {
                tracing::warn!(id = ?window_id, signal = ?signal, "Got a signal for an unknown window");
                continue;
            };

            let window = self.windows.get_mut(handle_id).unwrap();

            let WindowState::Rendering { handle, .. } = &mut window.state else {
                tracing::warn!(
                    window_id = ?handle_id, signal = ?signal,
                    "Tried to handle a signal whilst suspended or before window created"
                );
                return;
            };

            match signal {
                RenderRootSignal::Action(action, widget_id) => {
                    let window_id = window.id;
                    debug!("Action {:?} on widget {:?}", action, widget_id);
                    app_driver.on_action(
                        window_id,
                        &mut DriverCtx::new(self, event_loop),
                        widget_id,
                        action,
                    );
                }
                RenderRootSignal::StartIme => {
                    handle.set_ime_allowed(true);
                }
                RenderRootSignal::EndIme => {
                    handle.set_ime_allowed(false);
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    handle.set_ime_cursor_area(position, size);
                }
                RenderRootSignal::RequestRedraw => {
                    need_redraw.insert(*handle_id);
                }
                RenderRootSignal::RequestAnimFrame => {
                    // TODO
                    need_redraw.insert(*handle_id);
                }
                RenderRootSignal::TakeFocus => {
                    handle.focus_window();
                }
                RenderRootSignal::SetCursor(cursor) => {
                    handle.set_cursor(cursor);
                }
                RenderRootSignal::SetSize(size) => {
                    // TODO - Handle return value?
                    let _ = handle.request_inner_size(size);
                }
                RenderRootSignal::SetTitle(title) => {
                    handle.set_title(&title);
                }
                RenderRootSignal::DragWindow => {
                    // TODO - Handle return value?
                    let _ = handle.drag_window();
                }
                RenderRootSignal::DragResizeWindow(direction) => {
                    // TODO - Handle return value?
                    let direction = masonry_resize_direction_to_winit(direction);
                    let _ = handle.drag_resize_window(direction);
                }
                RenderRootSignal::ToggleMaximized => {
                    handle.set_maximized(!handle.is_maximized());
                }
                RenderRootSignal::Minimize => {
                    handle.set_minimized(true);
                }
                RenderRootSignal::Exit => {
                    event_loop.exit();
                }
                RenderRootSignal::ShowWindowMenu(position) => {
                    handle.show_window_menu(position);
                }
                RenderRootSignal::WidgetSelectedInInspector(widget_id) => {
                    let Some(widget) = window.render_root.get_widget(widget_id) else {
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

        // If an app creates a visible window, we firstly create it as invisible
        // and then render the first frame before making it visible to avoid flashing.
        for handle_id in self.need_first_frame.drain(0..) {
            let window = self.windows.get_mut(&handle_id).unwrap();
            let (scene, tree_update) = window.render_root.redraw();
            Self::render(
                self.surfaces.get_mut(&handle_id).unwrap(),
                window,
                scene,
                &self.render_cx,
                &mut self.renderer,
            );
            #[cfg(feature = "tracy")]
            drop(self.frame.take());
            if let WindowState::Rendering {
                handle,
                accesskit_adapter,
                ..
            } = &mut window.state
            {
                accesskit_adapter.update_if_active(|| tree_update);
                handle.set_visible(true);
            };
        }

        // If we're processing a lot of actions, we may have a lot of pending redraws.
        // We batch them up to avoid redundant requests.
        for handle_id in need_redraw {
            let window = self.windows.get(&handle_id).unwrap();
            let WindowState::Rendering { handle, .. } = &window.state else {
                unreachable!()
            };
            handle.request_redraw();
        }
    }

    fn handle_id(&self, window_id: WindowId) -> HandleId {
        *self
            .window_id_to_handle_id
            .get(&window_id)
            .unwrap_or_else(|| panic!("could not find window for id {window_id:?}"))
    }

    pub(crate) fn window(&self, window_id: WindowId) -> &Window {
        let handle_id = self.handle_id(window_id);
        self.windows.get(&handle_id).unwrap()
    }

    pub(crate) fn window_mut(&mut self, window_id: WindowId) -> &mut Window {
        let handle_id = self.handle_id(window_id);
        self.windows.get_mut(&handle_id).unwrap()
    }

    pub fn window_state(&self, window_id: WindowId) -> &WindowState {
        let handle_id = self.handle_id(window_id);
        &self.windows.get(&handle_id).unwrap().state
    }

    // TODO: remove (currently only exists to call register_fonts, font context should be moved out of render root)
    pub fn roots(&mut self) -> impl Iterator<Item = &mut RenderRoot> {
        self.windows
            .values_mut()
            .map(|window| &mut window.render_root)
    }

    pub fn set_present_mode(&mut self, window_id: WindowId, present_mode: wgpu::PresentMode) {
        let handle_id = self.handle_id(window_id);
        let surface = self.surfaces.get_mut(&handle_id).unwrap();
        self.render_cx.set_present_mode(surface, present_mode);
    }
}
