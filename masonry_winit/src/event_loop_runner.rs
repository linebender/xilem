// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, mpsc};

use accesskit_winit::Adapter;
use masonry_core::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry_core::core::keyboard::{Key, KeyState};
use masonry_core::core::{
    DefaultProperties, ErasedAction, NewWidget, TextEvent, Widget, WidgetId, WindowEvent,
};
use masonry_core::kurbo::Affine;
use masonry_core::peniko::Color;
use masonry_core::util::Instant;
use masonry_core::vello::util::{RenderContext, RenderSurface};
use masonry_core::vello::wgpu;
use masonry_core::vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use tracing::{debug, info, info_span};
use ui_events_winit::{WindowEventReducer, WindowEventTranslation};
use winit::application::ApplicationHandler;
use winit::cursor::Cursor;
use winit::dpi::Size;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent as WinitDeviceEvent, DeviceId, WindowEvent as WinitWindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{
    ImeRequestData, Window as WindowHandle, WindowAttributes, WindowId as HandleId,
};

use crate::app::{AppDriver, DriverCtx, masonry_resize_direction_to_winit, winit_ime_to_masonry};
use crate::app_driver::WindowId;

#[derive(Debug)]
pub enum MasonryUserEvent {
    AccessKit(HandleId, accesskit_winit::WindowEvent),
    // TODO: A more considered design here
    Action(WindowId, ErasedAction, WidgetId),
}

impl From<accesskit_winit::Event> for MasonryUserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::AccessKit(event.window_id, event.window_event)
    }
}

/// A container for a window yet to be created.
///
/// This is stored inside [`MasonryState`] and will be created during the `resumed` event.
pub struct NewWindow {
    /// The id is set by the App, and can be created using the [`WindowId::next()`] method.
    ///
    /// Once the window is created, it can be accessed using this `id` through the
    /// [`DriverCtx::window_handle()`] method.
    pub id: WindowId,
    /// Window attributes for the winit's [`Window`].
    ///
    /// A default attribute can be created using [`masonry_winit::winit::window::WindowAttributes::default()`].
    ///
    /// [`Window`]: crate::winit::window::Window
    /// [`masonry_winit::winit::window::WindowAttributes::default()`]: crate::winit::window::masonry_winit::winit::window::WindowAttributes::default()
    pub attributes: WindowAttributes,
    /// The widget which will take up the entire contents of the new window.
    pub root_widget: NewWidget<dyn Widget>,
    /// The base color of the window.
    pub base_color: Color,
}

impl NewWindow {
    /// Create a new window with an automatically assigned [`WindowId`].
    ///
    /// See the documentation on the fields of this type for details of the parameters.
    pub fn new(attributes: WindowAttributes, root_widget: NewWidget<dyn Widget + 'static>) -> Self {
        Self::new_with_id(WindowId::next(), attributes, root_widget)
    }

    /// Create a new window with a custom assigned [`WindowId`].
    ///
    /// Use this when you need to specify a unique ID for the window, for example,
    /// for external tracking or state management.
    pub fn new_with_id(
        id: WindowId,
        attributes: WindowAttributes,
        root_widget: NewWidget<dyn Widget + 'static>,
    ) -> Self {
        Self {
            id,
            attributes,
            root_widget,
            base_color: Color::BLACK,
        }
    }

    /// Set the base color of the new window.
    ///
    /// The base color is the color of the background which all widgets in the window draw on top of.
    /// Masonry's current default theme assumes that this will be a very dark color for sufficient contrast.
    /// This is most useful for apps which want to for example support light mode.
    ///
    /// Please note that it is not currently supported to modify this once the app is running.
    /// This is not a fundamental limitation, and is only due to missing api design.
    pub fn with_base_color(mut self, base_color: Color) -> Self {
        self.base_color = base_color;
        self
    }
}

/// Per-Window state
pub(crate) struct Window {
    id: WindowId,
    pub(crate) handle: Arc<dyn WindowHandle>,
    pub(crate) accesskit_adapter: Adapter,
    event_reducer: WindowEventReducer,
    pub(crate) render_root: RenderRoot,
    pub(crate) base_color: Color,
}

impl Window {
    pub(crate) fn new(
        window_id: WindowId,
        handle: Arc<dyn WindowHandle>,
        accesskit_adapter: Adapter,
        root_widget: NewWidget<dyn Widget>,
        signal_sender: Sender<(WindowId, RenderRootSignal)>,
        default_properties: Arc<DefaultProperties>,
        base_color: Color,
    ) -> Self {
        // TODO: We can't know this scale factor until later?
        let scale_factor = 1.0;

        Self {
            id: window_id,
            handle,
            accesskit_adapter,
            event_reducer: WindowEventReducer::default(),
            render_root: RenderRoot::new(
                root_widget,
                move |signal| {
                    signal_sender.clone().send((window_id, signal)).unwrap();
                },
                RenderRootOptions {
                    default_properties,
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    scale_factor,
                    test_font: None,
                },
            ),
            base_color,
        }
    }
}

/// The state of the Masonry application. If you run Masonry from an external Winit event loop, create a
/// `MasonryState` via [`MasonryState::new`] and forward events to it via the appropriate method (e.g.,
/// calling [`handle_window_event`](MasonryState::handle_window_event) in [`window_event`](ApplicationHandler::window_event)).
pub struct MasonryState<'a> {
    /// The event loop is suspended when the app is e.g. in the background on Android.
    /// We aren't allowed to have any `Surface`s, and we also don't expect to receive any events.
    is_suspended: bool,
    render_cx: RenderContext,
    renderer: Option<Renderer>,
    // TODO: Winit doesn't seem to let us create these proxies from within the loop
    // The reasons for this are unclear
    event_loop_proxy: EventLoopProxy,
    #[cfg(feature = "tracy")]
    frame: Option<tracing_tracy::client::Frame>,

    window_id_to_handle_id: HashMap<WindowId, HandleId>,

    surfaces: HashMap<HandleId, RenderSurface<'a>>,
    windows: HashMap<HandleId, Window>,

    // Is `Some` if the most recently displayed frame was an animation frame.
    last_anim: Option<Instant>,
    signal_receiver: mpsc::Receiver<(WindowId, RenderRootSignal)>,

    signal_sender: Sender<(WindowId, RenderRootSignal)>,
    default_properties: Arc<DefaultProperties>,
    pub(crate) exit: bool,
    /// Windows that are scheduled to be created in the next resumed event.
    new_windows: Vec<NewWindow>,
    need_first_frame: Vec<HandleId>,

    event_sender: Sender<MasonryUserEvent>,
    event_receiver: Receiver<MasonryUserEvent>,
}

struct MainState<'a> {
    masonry_state: MasonryState<'a>,
    app_driver: Box<dyn AppDriver>,
}

/// The type of the event loop used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoop = winit::event_loop::EventLoop;
/// The type of the event loop builder used by Masonry.
///
/// This *will* be changed to allow custom event types, but is implemented this way for expedience
pub type EventLoopBuilder = winit::event_loop::EventLoopBuilder;

/// A proxy used to send events to the event loop
pub type EventLoopProxy = winit::event_loop::EventLoopProxy;

// --- MARK: RUN
pub fn run(
    // Clearly, this API needs to be refactored, so we don't mind forcing this to be passed in here directly
    // This is passed in mostly to allow configuring the Android app
    mut loop_builder: EventLoopBuilder,
    event_sender: Sender<MasonryUserEvent>,
    event_receiver: Receiver<MasonryUserEvent>,
    new_windows: Vec<NewWindow>,
    app_driver: impl AppDriver + 'static,
    default_property_set: DefaultProperties,
) -> Result<(), EventLoopError> {
    let event_loop = loop_builder.build()?;

    run_with(
        event_loop,
        event_sender,
        event_receiver,
        new_windows,
        app_driver,
        default_property_set,
    )
}

pub fn run_with(
    event_loop: EventLoop,
    event_sender: Sender<MasonryUserEvent>,
    event_receiver: Receiver<MasonryUserEvent>,
    new_windows: Vec<NewWindow>,
    app_driver: impl AppDriver + 'static,
    default_properties: DefaultProperties,
) -> Result<(), EventLoopError> {
    // If there is no default tracing subscriber, we set our own. If one has
    // already been set, we get an error which we swallow.
    // By now, we're about to take control of the event loop. The user is unlikely
    // to try to set their own subscriber once the event loop has started.
    let _ = masonry_core::app::try_init_tracing();

    let mut main_state = MainState {
        masonry_state: MasonryState::new(
            event_loop.create_proxy(),
            new_windows,
            default_properties,
            event_sender,
            event_receiver,
        ),
        app_driver: Box::new(app_driver),
    };

    event_loop.run_app(&mut main_state)
}

impl ApplicationHandler for MainState<'_> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.masonry_state
            .handle_resumed(event_loop, &mut *self.app_driver);
    }

    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.masonry_state
            .handle_resumed(event_loop, &mut *self.app_driver);
    }

    fn suspended(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.masonry_state.handle_suspended(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
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
        event_loop: &dyn ActiveEventLoop,
        device_id: Option<DeviceId>,
        event: WinitDeviceEvent,
    ) {
        self.masonry_state.handle_device_event(
            event_loop,
            device_id,
            event,
            self.app_driver.as_mut(),
        );
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.masonry_state.event_receiver.try_recv() {
            self.masonry_state
                .handle_user_event(event_loop, event, self.app_driver.as_mut());
        }
    }

    // The following have empty handlers, but adding this here for future proofing. E.g., memory
    // warning is very likely to be handled for mobile and we in particular want to make sure
    // external event loops can let masonry handle these callbacks.

    fn about_to_wait(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_about_to_wait(event_loop);
    }

    fn new_events(
        &mut self,
        event_loop: &dyn winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.masonry_state.handle_new_events(event_loop, cause);
    }

    fn destroy_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_memory_warning(event_loop);
    }
}

impl MasonryState<'_> {
    pub fn new(
        event_loop_proxy: EventLoopProxy,
        new_windows: Vec<NewWindow>,
        default_properties: DefaultProperties,
        event_sender: Sender<MasonryUserEvent>,
        event_receiver: Receiver<MasonryUserEvent>,
    ) -> Self {
        let render_cx = RenderContext::new();

        let (signal_sender, signal_receiver) =
            std::sync::mpsc::channel::<(WindowId, RenderRootSignal)>();

        MasonryState {
            is_suspended: true,
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
            new_windows,
            need_first_frame: Vec::new(),

            event_sender,
            event_receiver,
        }
    }

    // --- MARK: RESUMED
    pub fn handle_resumed(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        app_driver: &mut dyn AppDriver,
    ) {
        if !self.is_suspended {
            // Short-circuiting since we have already
            // handled the resumed event before this.
            return;
        }

        self.is_suspended = false;

        //  Recreate surfaces for all existing windows.
        for (handle_id, window) in self.windows.iter() {
            let surface = create_surface(&mut self.render_cx, window.handle.clone());
            self.surfaces.insert(*handle_id, surface);
        }

        // Create new windows.
        if !self.new_windows.is_empty() {
            for new_window in std::mem::take(&mut self.new_windows) {
                self.create_window(event_loop, new_window);
            }
            // TODO: This is wrong in the case where the driver tries to create a window whilst suspended
            // The on_start would be called twice.
            app_driver.on_start(self);
        }

        self.handle_signals(event_loop, app_driver);
    }

    // --- MARK: SUSPENDED
    pub fn handle_suspended(&mut self, _event_loop: &dyn ActiveEventLoop) {
        if self.is_suspended {
            // Short-circuiting since we have already
            // handled the suspended event before this.
            return;
        }

        self.is_suspended = true;

        // All surfaces needs to be cleared when suspended.
        // They will be recreated when resumed.
        self.surfaces.clear();
    }

    pub(crate) fn create_window(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        new_window: NewWindow,
    ) {
        if self.window_id_to_handle_id.contains_key(&new_window.id) {
            panic!(
                "attempted to create a window with id {:?} but a window with that id already exists",
                new_window.id
            );
        }

        if self.is_suspended {
            // Wait until resumed before creating the windows.
            self.new_windows.push(new_window);

            return;
        }

        let visible = new_window.attributes.visible;
        // We always create the window as invisible so that we can
        // render the first frame before showing it to avoid flashing.
        let handle = event_loop
            .create_window(new_window.attributes.with_visible(false))
            .unwrap();
        if visible {
            // We defer the rendering of the first frame to the handle_signals method because
            // we want to handle any signals caused by the initial layout or rescale before we render.
            self.need_first_frame.push(handle.id());
        }

        let adapter = Adapter::with_event_loop_proxy(
            event_loop,
            handle.as_ref(),
            self.event_sender.clone(),
            self.event_loop_proxy.clone(),
        );

        let handle: Arc<dyn WindowHandle> = Arc::from(handle);

        let scale_factor = handle.scale_factor();
        let handle_id = handle.id();

        let surface = create_surface(&mut self.render_cx, handle.clone());
        self.surfaces.insert(handle_id, surface);

        let mut window = Window::new(
            new_window.id,
            handle,
            adapter,
            new_window.root_widget,
            self.signal_sender.clone(),
            self.default_properties.clone(),
            new_window.base_color,
        );
        window
            .render_root
            .handle_window_event(WindowEvent::Rescale(scale_factor));

        tracing::debug!(window_id = window.id.trace(), handle=?handle_id, "creating window");
        self.window_id_to_handle_id.insert(window.id, handle_id);
        self.windows.insert(handle_id, window);
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
        #[allow(deprecated)]
        window.handle.set_ime_allowed(false);
    }

    // --- MARK: RENDER
    fn render(
        surface: &mut RenderSurface<'_>,
        window: &mut Window,
        scene: Scene,
        render_cx: &RenderContext,
        renderer: &mut Option<Renderer>,
    ) {
        let scale_factor = window.handle.scale_factor();
        // https://github.com/rust-windowing/winit/issues/2308
        #[cfg(target_os = "ios")]
        let size = window.handle.outer_size();
        #[cfg(not(target_os = "ios"))]
        let size = window.handle.surface_size();
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
            base_color: window.base_color,
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
        window.handle.pre_present_notify();
        surface_texture.present();
        {
            let _render_poll_span =
                tracing::info_span!("Waiting for GPU to finish rendering").entered();
            device.poll(wgpu::PollType::Wait).unwrap();
        }
    }

    // --- MARK: WINDOW_EVENT
    pub fn handle_window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        handle_id: HandleId,
        event: WinitWindowEvent,
        app_driver: &mut dyn AppDriver,
    ) {
        if self.is_suspended {
            tracing::warn!(
                ?event,
                "Got window event whilst suspended or before window created"
            );
            return;
        };

        let Some(window) = self.windows.get_mut(&handle_id) else {
            tracing::warn!(
                ?event,
                "Got window event for unknown window {:?}",
                handle_id
            );
            return;
        };

        let _span = info_span!("window_event", window_id = window.id.trace()).entered();
        #[cfg(feature = "tracy")]
        if self.frame.is_none() {
            self.frame = Some(tracing_tracy::client::non_continuous_frame!("Masonry"));
        }
        window
            .accesskit_adapter
            .process_event(window.handle.as_ref(), &event);

        if !matches!(
            event,
            WinitWindowEvent::KeyboardInput {
                is_synthetic: true,
                ..
            }
        ) && let Some(wet) = window.event_reducer.reduce(&event)
        {
            match wet {
                WindowEventTranslation::Keyboard(k) => {
                    // TODO - Detect in Masonry code instead
                    let action_mod = if cfg!(target_os = "macos") {
                        k.modifiers.meta()
                    } else {
                        k.modifiers.ctrl()
                    };
                    if let Key::Character(c) = &k.key
                        && c.as_str().eq_ignore_ascii_case("v")
                        && action_mod
                        && k.state == KeyState::Down
                    {
                    } else {
                        window.render_root.handle_text_event(TextEvent::Keyboard(k));
                    }
                }
                WindowEventTranslation::Pointer(p) => {
                    window.render_root.handle_pointer_event(p);
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
                if let Some(tree_update) = tree_update {
                    window.accesskit_adapter.update_if_active(|| tree_update);
                }
            }
            WinitWindowEvent::CloseRequested => {
                app_driver.on_close_requested(window.id, &mut DriverCtx::new(self, event_loop));
            }
            WinitWindowEvent::SurfaceResized(size) => {
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
        _: &dyn ActiveEventLoop,
        _: Option<DeviceId>,
        _: WinitDeviceEvent,
        _: &mut dyn AppDriver,
    ) {
    }

    // --- MARK: USER_EVENT
    pub fn handle_user_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
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
                            .handle_window_event(WindowEvent::EnableAccessTree);
                    }
                    accesskit_winit::WindowEvent::ActionRequested(action_request) => {
                        state.render_root.handle_access_event(action_request);
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                        state
                            .render_root
                            .handle_window_event(WindowEvent::DisableAccessTree);
                    }
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
    pub fn handle_about_to_wait(&mut self, _: &dyn ActiveEventLoop) {}

    pub fn handle_new_events(&mut self, _: &dyn ActiveEventLoop, _: winit::event::StartCause) {}

    pub fn handle_exiting(&mut self, _: &dyn ActiveEventLoop) {}

    pub fn handle_memory_warning(&mut self, _: &dyn ActiveEventLoop) {}

    // --- MARK: SIGNALS
    fn handle_signals(&mut self, event_loop: &dyn ActiveEventLoop, app_driver: &mut dyn AppDriver) {
        if self.is_suspended {
            tracing::warn!("Tried to handle a signal whilst suspended");
            return;
        }

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
            let handle = &window.handle;

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
                    #[allow(deprecated)]
                    handle.set_ime_allowed(true);
                }
                RenderRootSignal::EndIme => {
                    #[allow(deprecated)]
                    handle.set_ime_allowed(false);
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    handle
                        .request_ime_update(winit::window::ImeRequest::Update(
                            ImeRequestData::default().with_cursor_area(
                                winit::dpi::Position::Logical(position),
                                winit::dpi::Size::Logical(size),
                            ),
                        ))
                        .unwrap();
                }
                RenderRootSignal::ClipboardStore(_) => {}
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
                    handle.set_cursor(Cursor::Icon(cursor));
                }
                RenderRootSignal::SetSize(size) => {
                    // TODO - Handle return value?
                    let _ = handle.request_surface_size(Size::Physical(size));
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
                    handle.show_window_menu(winit::dpi::Position::Logical(position));
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

            if let Some(tree_update) = tree_update {
                window.accesskit_adapter.update_if_active(|| tree_update);
            }

            window.handle.set_visible(true);
        }

        // If we're processing a lot of actions, we may have a lot of pending redraws.
        // We batch them up to avoid redundant requests.
        for handle_id in need_redraw {
            let window = self.windows.get(&handle_id).unwrap();
            window.handle.request_redraw();
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

    pub fn is_suspended(&self) -> bool {
        self.is_suspended
    }

    // TODO: Remove this method.
    // It's currently used to call register_fonts and set_focus_fallback.
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

fn create_surface<'s>(
    render_cx: &mut RenderContext,
    handle: Arc<dyn WindowHandle>,
) -> RenderSurface<'s> {
    // https://github.com/rust-windowing/winit/issues/2308
    #[cfg(target_os = "ios")]
    let size = handle.outer_size();
    #[cfg(not(target_os = "ios"))]
    let size = handle.surface_size();

    pollster::block_on(render_cx.create_surface(
        handle.clone(),
        size.width,
        size.height,
        wgpu::PresentMode::AutoVsync,
    ))
    .unwrap()
}
