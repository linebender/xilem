// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tools and infrastructure for testing widgets.

use std::num::NonZeroUsize;

use image::{ImageReader, Rgba, RgbaImage};
use vello::util::RenderContext;
use vello::{block_on_wgpu, RendererOptions};
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, ImageCopyBuffer,
    TextureDescriptor, TextureFormat, TextureUsages,
};
use winit::event::Ime;

use super::screenshots::get_image_diff;
use super::snapshot_utils::get_cargo_workspace;
use crate::action::Action;
use crate::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
use crate::event::{PointerButton, PointerEvent, PointerState, TextEvent, WindowEvent};
use crate::render_root::{RenderRoot, RenderRootSignal, WindowSizePolicy};
use crate::tracing_backend::try_init_tracing;
use crate::widget::{WidgetMut, WidgetRef};
use crate::{Color, Handled, Point, Size, Vec2, Widget, WidgetId};

// TODO - Get shorter names
// TODO - Make them associated consts
/// Default canvas size for tests.
pub const HARNESS_DEFAULT_SIZE: Size = Size::new(400., 400.);

/// Default background color for tests.
pub const HARNESS_DEFAULT_BACKGROUND_COLOR: Color = Color::rgb8(0x29, 0x29, 0x29);

/// A safe headless environment to test widgets in.
///
/// `TestHarness` is a type that simulates an [`AppRoot`](crate::AppRoot)
/// with a single window.
///
/// ## Workflow
///
/// One of the main goals of masonry is to provide primitives that allow application
/// developers to test their app in a convenient and intuitive way. The basic testing
/// workflow is as follows:
///
/// - Create a harness with some widget.
/// - Send events to the widget as if you were a user interacting with a window.
///   (Lifecycle and layout passes are handled automatically.)
/// - Check that the state of the widget graph matches what you expect.
///
/// You can do that last part in a few different ways. You can get a [`WidgetRef`] to
/// a specific widget through methods like [`try_get_widget`](Self::try_get_widget). [`WidgetRef`] implements
/// `Debug`, so you can check the state of an entire tree with something like the `insta`
/// crate.
///
/// You can also render the widget tree directly with the [`render`](Self::render) method. Masonry also
/// provides the [`assert_render_snapshot`] macro, which performs snapshot testing on the
/// rendered widget tree automatically.
///
/// ## Fidelity
///
/// `TestHarness` tries to act like the normal masonry environment. For instance, it will dispatch every `Command` sent during event handling, handle lifecycle methods, etc.
///
/// The passage of time is simulated with the [`move_timers_forward`](Self::move_timers_forward) methods. **(TODO -
/// Doesn't move animations forward.)**
///
/// **(TODO - ExtEvents aren't handled.)**
///
/// **(TODO - Painting invalidation might not be accurate.)**
///
/// One minor difference is that layout is always calculated after every event, whereas
/// in normal execution it is only calculated before paint. This might be create subtle
/// differences in cases where timers are programmed to fire at the same time: in normal
/// execution, they'll execute back-to-back; in the harness, they'll be separated with
/// layout calls.
///
/// Also, paint only happens when the user explicitly calls rendering methods, whereas in
/// a normal applications you could reasonably expect multiple paint calls between eg any
/// two clicks.
///
/// ## Example
///
/// ```
/// use insta::assert_debug_snapshot;
///
/// use masonry::PointerButton;
/// use masonry::widget::Button;
/// use masonry::Action;
/// use masonry::assert_render_snapshot;
/// use masonry::testing::widget_ids;
/// use masonry::testing::TestHarness;
/// use masonry::testing::TestWidgetExt;
/// use masonry::theme::PRIMARY_LIGHT;
///
/// # /*
/// #[test]
/// # */
/// fn simple_button() {
///     let [button_id] = widget_ids();
///     let widget = Button::new("Hello").with_id(button_id);
///
///     let mut harness = TestHarness::create(widget);
///
///     # if false {
///     assert_debug_snapshot!(harness.root_widget());
///     assert_render_snapshot!(harness, "hello");
///     # }
///
///     assert_eq!(harness.pop_action(), None);
///
///     harness.mouse_click_on(button_id);
///     assert_eq!(
///         harness.pop_action(),
///         Some((Action::ButtonPressed(PointerButton::Primary), button_id))
///     );
/// }
///
/// # simple_button();
/// ```
pub struct TestHarness {
    render_root: RenderRoot,
    mouse_state: PointerState,
    window_size: PhysicalSize<u32>,
    background_color: Color,
}

/// Assert a snapshot of a rendered frame of your app.
///
/// This macro takes a test harness and a name, renders the current state of the app,
/// and stores the render as a PNG next to the text, in a `./screenshots/` folder.
///
/// If a screenshot already exists, the rendered value is compared against this screenshot.
/// The assert passes if both are equal; otherwise, a diff file is created.
///
/// If a screenshot doesn't exist, the assert will fail; the new screenshot is stored as
/// `./screenshots/<test_name>.new.png`, and must be renamed before the assert will pass.
#[macro_export]
macro_rules! assert_render_snapshot {
    ($test_harness:expr, $name:expr) => {
        $test_harness.check_render_snapshot(
            env!("CARGO_MANIFEST_DIR"),
            file!(),
            module_path!(),
            $name,
        )
    };
}

impl TestHarness {
    /// Builds harness with given root widget.
    ///
    /// Window size will be [`HARNESS_DEFAULT_SIZE`].
    /// Background color will be [`HARNESS_DEFAULT_BACKGROUND_COLOR`].
    pub fn create(root_widget: impl Widget) -> Self {
        Self::create_with(
            root_widget,
            HARNESS_DEFAULT_SIZE,
            HARNESS_DEFAULT_BACKGROUND_COLOR,
        )
    }

    // TODO - Remove
    /// Builds harness with given root widget and window size.
    pub fn create_with_size(root_widget: impl Widget, window_size: Size) -> Self {
        Self::create_with(root_widget, window_size, HARNESS_DEFAULT_BACKGROUND_COLOR)
    }

    /// Builds harness with given root widget, canvas size and background color.
    pub fn create_with(
        root_widget: impl Widget,
        window_size: Size,
        background_color: Color,
    ) -> Self {
        let mouse_state = PointerState::empty();
        let window_size = PhysicalSize::new(window_size.width as _, window_size.height as _);

        // If there is no default tracing subscriber, we set our own. If one has
        // already been set, we get an error which we swallow.
        // Having a default subscriber is helpful for tests; swallowing errors means
        // we don't panic if the user has already set one, or a test creates multiple
        // harnesses.
        let _ = try_init_tracing();

        let mut harness = TestHarness {
            render_root: RenderRoot::new(root_widget, WindowSizePolicy::User, 1.0),
            mouse_state,
            window_size,
            background_color,
        };
        harness.process_window_event(WindowEvent::Resize(window_size));

        harness
    }

    // --- MARK: PROCESS EVENTS ---
    // FIXME - The docs for these three functions are copy-pasted. Rewrite them.

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. Commands created as a result of this event
    /// will also be dispatched.
    pub fn process_window_event(&mut self, event: WindowEvent) -> Handled {
        let handled = self.render_root.handle_window_event(event);
        self.process_state_after_event();
        handled
    }

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. Commands created as a result of this event
    /// will also be dispatched.
    pub fn process_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let handled = self.render_root.handle_pointer_event(event);
        self.process_state_after_event();
        handled
    }

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. Commands created as a result of this event
    /// will also be dispatched.
    pub fn process_text_event(&mut self, event: TextEvent) -> Handled {
        let handled = self.render_root.handle_text_event(event);
        self.process_state_after_event();
        handled
    }

    fn process_state_after_event(&mut self) {
        if self.root_widget().state().needs_layout {
            self.render_root.root_layout();
        }
    }

    // --- MARK: RENDER ---
    // TODO - We add way too many dependencies in this code
    // TODO - Should be async?
    /// Create a bitmap (an array of pixels), paint the window and return the bitmap as an 8-bits-per-channel RGB image.
    pub fn render(&mut self) -> RgbaImage {
        let (scene, _tree_update) = self.render_root.redraw();
        if std::env::var("SKIP_RENDER_TESTS").is_ok_and(|it| !it.is_empty()) {
            return RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
        }
        let mut context = RenderContext::new();
        let device_id =
            pollster::block_on(context.device(None)).expect("No compatible device found");
        let device_handle = &mut context.devices[device_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;
        let mut renderer = vello::Renderer::new(
            device,
            RendererOptions {
                surface_format: None,
                // TODO - Examine this value
                use_cpu: true,
                num_init_threads: NonZeroUsize::new(1),
                // TODO - Examine this value
                antialiasing_support: vello::AaSupport::area_only(),
            },
        )
        .expect("Got non-Send/Sync error from creating renderer");

        // TODO - fix window_size
        let (width, height) = (self.window_size.width, self.window_size.height);
        let render_params = vello::RenderParams {
            // TODO - Parameterize
            base_color: self.background_color,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
            debug: vello::DebugLayers::none(),
        };

        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = device.create_texture(&TextureDescriptor {
            label: Some("Target texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        renderer
            .render_to_texture(device, queue, &scene, &view, &render_params)
            .expect("Got non-Send/Sync error from rendering");
        let padded_byte_width = (width * 4).next_multiple_of(256);
        let buffer_size = padded_byte_width as u64 * height as u64;
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("val"),
            size: buffer_size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Copy out buffer"),
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_byte_width),
                    rows_per_image: None,
                },
            },
            size,
        );

        queue.submit([encoder.finish()]);
        let buf_slice = buffer.slice(..);

        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buf_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        let recv_result = block_on_wgpu(device, receiver.receive()).expect("channel was closed");
        recv_result.expect("failed to map buffer");

        let data = buf_slice.get_mapped_range();
        let mut result_unpadded =
            Vec::<u8>::with_capacity((width * height * 4).try_into().unwrap());
        for row in 0..height {
            let start = (row * padded_byte_width).try_into().unwrap();
            result_unpadded.extend(&data[start..start + (width * 4) as usize]);
        }

        RgbaImage::from_vec(width, height, result_unpadded).expect("failed to create image")
    }

    // --- MARK: EVENT HELPERS ---

    /// Move an internal mouse state, and send a [`PointerMove`](PointerEvent::PointerMove) event to the window.
    pub fn mouse_move(&mut self, pos: impl Into<Point>) {
        // FIXME - Account for scaling
        let pos = pos.into();
        let pos = PhysicalPosition::new(pos.x, pos.y);
        self.mouse_state.physical_position = dbg!(pos);
        // TODO: may want to support testing with non-unity scale factors.
        let scale_factor = 1.0;
        self.mouse_state.position = pos.to_logical(scale_factor);

        self.process_pointer_event(PointerEvent::PointerMove(self.mouse_state.clone()));
    }

    /// Send a [`PointerDown`](PointerEvent::PointerDown) event to the window.
    pub fn mouse_button_press(&mut self, button: PointerButton) {
        self.mouse_state.buttons.insert(button);
        self.process_pointer_event(PointerEvent::PointerDown(button, self.mouse_state.clone()));
    }

    /// Send a [`PointerUp`](PointerEvent::PointerUp) event to the window.
    pub fn mouse_button_release(&mut self, button: PointerButton) {
        self.mouse_state.buttons.remove(&button);
        self.process_pointer_event(PointerEvent::PointerUp(button, self.mouse_state.clone()));
    }

    /// Send a [`MouseWheel`](PointerEvent::MouseWheel) event to the window.
    pub fn mouse_wheel(&mut self, wheel_delta: Vec2) {
        let pixel_delta = LogicalPosition::new(wheel_delta.x, wheel_delta.y);
        self.process_pointer_event(PointerEvent::MouseWheel(
            pixel_delta,
            self.mouse_state.clone(),
        ));
    }

    /// Send events that lead to a given widget being clicked.
    ///
    /// Combines [`mouse_move`](Self::mouse_move), [`mouse_button_press`](Self::mouse_button_press), and [`mouse_button_release`](Self::mouse_button_release).
    pub fn mouse_click_on(&mut self, id: WidgetId) {
        let widget_rect = self.get_widget(id).state().window_layout_rect();
        let widget_center = widget_rect.center();

        self.mouse_move(widget_center);
        self.mouse_button_press(PointerButton::Primary);
        self.mouse_button_release(PointerButton::Primary);
    }

    /// Use [`mouse_move`](Self::mouse_move) to set the internal mouse pos to the center of the given widget.
    pub fn mouse_move_to(&mut self, id: WidgetId) {
        // FIXME - handle case where the widget isn't visible
        // FIXME - assert that the widget correctly receives the event otherwise?
        let widget_rect = self.get_widget(id).state().window_layout_rect();
        let widget_center = widget_rect.center();

        self.mouse_move(widget_center);
    }

    // TODO - Handle complicated IME
    // TODO - Mock Winit keyboard events
    pub fn keyboard_type_chars(&mut self, text: &str) {
        // For each character
        for c in text.split("").filter(|s| !s.is_empty()) {
            let event = TextEvent::Ime(Ime::Commit(c.to_string()));
            self.render_root.handle_text_event(event);
        }
        self.process_state_after_event();
    }

    #[cfg(FALSE)]
    /// Simulate the passage of time.
    ///
    /// If you create any timer in a widget, this method is the only way to trigger
    /// them in unit tests. The testing model assumes that everything else executes
    /// instantly, and timers are never triggered "spontaneously".
    ///
    /// **(TODO - Doesn't move animations forward.)**
    pub fn move_timers_forward(&mut self, duration: Duration) {
        // TODO - handle animations
        let tokens = self
            .mock_app
            .window
            .mock_timer_queue
            .as_mut()
            .unwrap()
            .move_forward(duration);
        for token in tokens {
            self.process_event(Event::Timer(token));
        }
    }

    // --- MARK: GETTERS ---

    /// Return the root widget.
    pub fn root_widget(&self) -> WidgetRef<'_, dyn Widget> {
        self.render_root.get_root_widget()
    }

    /// Return the widget with the given id.
    ///
    /// ## Panics
    ///
    /// Panics if no Widget with this id can be found.
    pub fn get_widget(&self, id: WidgetId) -> WidgetRef<'_, dyn Widget> {
        self.render_root
            .get_widget(id)
            .unwrap_or_else(|| panic!("could not find widget #{}", id.to_raw()))
    }

    /// Try to return the widget with the given id.
    pub fn try_get_widget(&self, id: WidgetId) -> Option<WidgetRef<'_, dyn Widget>> {
        self.render_root.get_widget(id)
    }

    // TODO - link to focus documentation.
    /// Return the widget that receives keyboard events.
    pub fn focused_widget(&self) -> Option<WidgetRef<'_, dyn Widget>> {
        self.root_widget()
            .find_widget_by_id(self.render_root.state.focused_widget?)
    }

    /// Call the provided visitor on every widget in the widget tree.
    pub fn inspect_widgets(&mut self, f: impl Fn(WidgetRef<'_, dyn Widget>) + 'static) {
        fn inspect(
            widget: WidgetRef<'_, dyn Widget>,
            f: &(impl Fn(WidgetRef<'_, dyn Widget>) + 'static),
        ) {
            f(widget);
            for child in widget.children() {
                inspect(child, f);
            }
        }

        inspect(self.root_widget(), &f);
    }

    /// Get a [`WidgetMut`] to the root widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_root_widget<R>(
        &mut self,
        f: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
    ) -> R {
        let res = self.render_root.edit_root_widget(f);
        self.process_state_after_event();
        res
    }

    /// Pop next action from the queue
    ///
    /// Note: Actions are still a WIP feature.
    pub fn pop_action(&mut self) -> Option<(Action, WidgetId)> {
        let signal = self
            .render_root
            .pop_signal_matching(|signal| matches!(signal, RenderRootSignal::Action(..)));
        match signal {
            Some(RenderRootSignal::Action(action, id)) => Some((action, id)),
            Some(_) => unreachable!(),
            _ => None,
        }
    }

    // --- MARK: SNAPSHOT ---

    /// Method used by [`assert_render_snapshot`]. Use the macro instead.
    ///
    /// Renders the current Widget tree to a pixmap, and compares the pixmap against the
    /// snapshot stored in `./screenshots/module_path__test_name.png`.
    ///
    /// * **manifest_dir:** directory where `Cargo.toml` can be found.
    /// * **test_file_path:** file path the current test is in.
    /// * **test_module_path:** import path of the module the current test is in.
    /// * **test_name:** arbitrary name; second argument of assert_render_snapshot.
    pub fn check_render_snapshot(
        &mut self,
        manifest_dir: &str,
        test_file_path: &str,
        test_module_path: &str,
        test_name: &str,
    ) {
        if option_env!("SKIP_RENDER_SNAPSHOTS").is_some() {
            // FIXME - This is a terrible, awful hack.
            // We need a way to skip render snapshots on CI and locally
            // until we can make sure the snapshots render the same on
            // different platforms.

            // We still redraw to get some coverage in the paint code.
            let _ = self.render_root.redraw();

            return;
        }

        let new_image = self.render();

        let workspace_path = get_cargo_workspace(manifest_dir);
        let test_file_path_abs = workspace_path.join(test_file_path);
        let folder_path = test_file_path_abs.parent().unwrap();

        let screenshots_folder = folder_path.join("screenshots");
        std::fs::create_dir_all(&screenshots_folder).unwrap();

        let module_str = test_module_path.replace("::", "__");

        let reference_path = screenshots_folder.join(format!("{module_str}__{test_name}.png"));
        let new_path = screenshots_folder.join(format!("{module_str}__{test_name}.new.png"));
        let diff_path = screenshots_folder.join(format!("{module_str}__{test_name}.diff.png"));

        if let Ok(reference_file) = ImageReader::open(reference_path) {
            let ref_image = reference_file.decode().unwrap().to_rgba8();

            if let Some(diff_image) = get_image_diff(&ref_image, &new_image) {
                // Remove '<test_name>.new.png' '<test_name>.diff.png' files if they exist
                let _ = std::fs::remove_file(&new_path);
                let _ = std::fs::remove_file(&diff_path);
                new_image.save(&new_path).unwrap();
                diff_image.save(&diff_path).unwrap();
                panic!("Images are different");
            }
        } else {
            // Remove '<test_name>.new.png' file if it exists
            let _ = std::fs::remove_file(&new_path);
            new_image.save(&new_path).unwrap();
            panic!("No reference file");
        }
    }

    // --- Debug logger ---

    // ex: harness.write_debug_logs("test_log.json");
    #[allow(missing_docs)]
    pub fn write_debug_logs(&mut self, path: &str) {
        self.render_root.state.debug_logger.write_to_file(path);
    }
}
