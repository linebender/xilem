// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tools and infrastructure for testing widgets.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use image::{DynamicImage, ImageFormat, ImageReader, Rgba, RgbaImage};
use masonry_core::accesskit::{Action, ActionRequest, Node, Role, Tree, TreeUpdate};
use masonry_core::anymore::AnyDebug;
use masonry_core::core::keyboard::{Code, Key, KeyState, NamedKey};
use masonry_core::core::pointer::{PointerButtonEvent, PointerScrollEvent};
use oxipng::{Options, optimize_from_memory};
use tracing::debug;

use masonry_core::app::{
    RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy, try_init_test_tracing,
};
use masonry_core::core::{
    CursorIcon, DefaultProperties, ErasedAction, FromDynWidget, Handled, Ime, KeyboardEvent,
    Modifiers, NewWidget, PointerButton, PointerEvent, PointerId, PointerInfo, PointerState,
    PointerType, PointerUpdate, ScrollDelta, TextEvent, Widget, WidgetId, WidgetMut, WidgetRef,
    WidgetTag, WindowEvent,
};
use masonry_core::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use masonry_core::kurbo::{Point, Size, Vec2};
use masonry_core::peniko::{Blob, Color};
use masonry_core::util::Duration;
use masonry_core::vello;
use masonry_core::vello::util::{RenderContext, block_on_wgpu};
use masonry_core::vello::wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use crate::screenshots::get_image_diff;
use crate::{Record, Recorder};

/// A [`PointerInfo`] for a primary mouse, for testing.
pub const PRIMARY_MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

// TODO - Add kittest support
// - Being able to check that the tree has an access node
// - Getting a WidgetRef/WidgetMut to a node from a kittest::Queryable
// - Making a debug snapshot of the access tree
// https://github.com/rerun-io/kittest

// TODO - Re-enable doc test.
// Doc test is currently disabled because it depends on a parent crate.

/// A safe headless environment to test widgets in.
///
/// `TestHarness` is a type that simulates a [`RenderRoot`] for testing.
///
/// # Workflow
///
/// One of the main goals of Masonry is to provide primitives that allow application
/// developers to test their app in a convenient and intuitive way.
/// The basic testing workflow is as follows:
///
/// - Create a harness with some widget.
/// - Send events to the widget as if you were a user interacting with a window.
///   (Rewrite passes are handled automatically.)
/// - Check that the state of the widget graph matches what you expect.
///
/// You can do that last part in a few different ways.
/// You can get a [`WidgetRef`] to a specific widget through methods like [`try_get_widget`](Self::try_get_widget).
/// [`WidgetRef`] implements `Debug`, so you can check the state of an entire tree with something like the [`insta`] crate.
///
/// You can also render the widget tree directly with the [`render`](Self::render) method.
/// Masonry also provides the [`assert_render_snapshot`] macro, which performs snapshot testing on the
/// rendered widget tree automatically.
///
/// # Fidelity
///
/// `TestHarness` tries to act like the normal masonry environment. It will run the same passes as the normal app after every user event and animation.
///
/// Animations can be simulated with the [`animate_ms`](Self::animate_ms) method.
///
/// One minor difference is that paint only happens when the user explicitly calls rendering
/// methods, whereas in a normal applications you could reasonably expect multiple paint calls
/// between eg any two clicks.
///
/// # Example
///
/// ```rust,ignore
/// use insta::assert_debug_snapshot;
///
/// use masonry::core::PointerButton;
/// use masonry::core::Action;
/// use masonry::testing::assert_render_snapshot;
/// use masonry::testing::widget_ids;
/// use masonry::testing::TestHarness;
/// use masonry::testing::TestWidgetExt;
/// use masonry::theme::default_property_set;
/// use masonry::widgets::Button;
/// # /*
/// #[test]
/// # */
/// fn simple_button() {
///     let [button_id] = widget_ids();
///     let widget = Button::new("Hello").with_id(button_id);
///
///     let mut harness = TestHarness::create(default_property_set(), widget);
///
///     # if false {
///     assert_render_snapshot!(harness, "hello");
///     # }
///
///     assert_eq!(harness.pop_action(), None);
///
///     harness.mouse_click_on(button_id);
///     assert_eq!(
///         harness.pop_action(),
///         Some((Action::ButtonPressed(Some(PointerButton::Primary)), button_id))
///     );
/// }
///
/// # simple_button();
/// ```
///
/// [`assert_render_snapshot`]: crate::assert_render_snapshot
/// [`insta`]: https://docs.rs/insta/latest/insta/
pub struct TestHarness<W: Widget> {
    signal_receiver: mpsc::Receiver<RenderRootSignal>,
    render_root: RenderRoot,
    access_tree: accesskit_consumer::Tree,
    render_context: Option<RenderContext>,
    vello_renderer: Option<vello::Renderer>,
    mouse_state: PointerState,
    window_size: PhysicalSize<u32>,
    background_color: Color,
    panic_on_rewrite_saturation: bool,
    screenshot_tolerance: u32,
    max_screenshot_size: u32,
    action_queue: VecDeque<(ErasedAction, WidgetId)>,
    has_ime_session: bool,
    ime_rect: (LogicalPosition<f64>, LogicalSize<f64>),
    clipboard: String,
    title: String,
    _marker: PhantomData<W>,
}

/// Parameters for creating a [`TestHarness`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TestHarnessParams {
    /// The size of the virtual window the harness renders into for snapshot testing.
    /// Defaults to [`Self::DEFAULT_SIZE`].
    pub window_size: Size,
    /// The background color of the virtual window.
    /// Defaults to [`Self::DEFAULT_BACKGROUND_COLOR`].
    pub background_color: Color,
    /// The maximum difference between two pixel channels before the harness will fail a screenshot test.
    /// Defaults to [`Self::DEFAULT_SCREENSHOT_TOLERANCE`].
    pub screenshot_tolerance: u32,
    /// The scale factor widgets are rendered at.
    /// Defaults to 1.0.
    pub scale_factor: f64,
    /// Whether to panic when we detect a loop in [rewrite passes](masonry_core::doc::pass_system#rewrite-passes).
    ///
    /// A loop means a case where the passes keep running because some passes keep
    /// invalidating flags for previous passes.
    pub panic_on_rewrite_saturation: bool,
    /// The largest size a screenshot file is allowed to be in this test.
    /// Defaults to `8KiB`.
    ///
    /// You can use [`TestHarnessParams::KIBIBYTE`] to help set this.
    /// Keeping screenshot files small avoids clones of this repository taking too long.
    /// Masonry testing uses [oxipng] to optimise the size of screenshots.
    pub max_screenshot_size: u32,
}

/// Assert a snapshot of a rendered frame of your app.
///
/// This macro takes a test harness and a name, renders the current state of the app,
/// and stores the rendered image to `<CRATE-ROOT>/screenshots/<TEST-NAME>.png`.
///
/// If a screenshot already exists, the rendered value is compared against this screenshot.
/// The assert passes if both are equal; otherwise, a diff file is created.
/// If the test is run again and the new rendered value matches the old screenshot, the diff file is deleted.
///
/// If a screenshot doesn't exist, the assert will fail; the new screenshot is stored as
/// `<CRATE-ROOT>/screenshots/<TEST-NAME>.new.png`, and must be renamed before the assert will pass.
///
/// You can also run tests with the `MASONRY_TEST_BLESS` flag set to `1` to assume all
/// differences are intended and overwrite all the screenshots with new values.
#[macro_export]
macro_rules! assert_render_snapshot {
    ($test_harness:expr, $name:expr) => {
        $test_harness.check_render_snapshot(env!("CARGO_MANIFEST_DIR"), $name, false)
    };
}

/// Assert a snapshot of a rendered frame of your app, expecting it to fail.
///
/// This macro does essentially the same thing as [`assert_render_snapshot`], but
/// instead of asserting that the rendered frame matches the existing screenshot,
/// it asserts that it does not match.
///
/// This is mostly used internally by Masonry to test that the image diffing does
/// detect changes and regressions.
///
/// This macro is read-only and will not write any new screenshots.
///
/// [`assert_render_snapshot`]: crate::assert_render_snapshot
#[macro_export]
macro_rules! assert_failing_render_snapshot {
    ($test_harness:expr, $name:expr) => {
        $test_harness.check_render_snapshot(env!("CARGO_MANIFEST_DIR"), $name, true)
    };
}

impl TestHarnessParams {
    /// Default test param values.
    pub const DEFAULT: Self = Self {
        window_size: Self::DEFAULT_SIZE,
        background_color: Self::DEFAULT_BACKGROUND_COLOR,
        screenshot_tolerance: Self::DEFAULT_SCREENSHOT_TOLERANCE,
        scale_factor: 1.0,
        panic_on_rewrite_saturation: true,
        max_screenshot_size: 8 * Self::KIBIBYTE,
    };

    /// Default canvas size for tests.
    pub const DEFAULT_SIZE: Size = Size::new(400., 400.);

    /// Default error tolerance for screenshot tests.
    pub const DEFAULT_SCREENSHOT_TOLERANCE: u32 = 16;

    /// Default background color for tests.
    pub const DEFAULT_BACKGROUND_COLOR: Color = Color::from_rgb8(0x29, 0x29, 0x29);

    /// One kibibyte. Used in [`TestHarnessParams::max_screenshot_size`].
    pub const KIBIBYTE: u32 = 1024;
}

impl Default for TestHarnessParams {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<W: Widget> TestHarness<W> {
    // -- MARK: CREATE
    /// Builds harness with given root widget.
    ///
    /// Window size will be [`TestHarnessParams::DEFAULT_SIZE`].
    /// Background color will be [`TestHarnessParams::DEFAULT_BACKGROUND_COLOR`].
    pub fn create(default_props: DefaultProperties, root_widget: NewWidget<W>) -> Self {
        Self::create_with(default_props, root_widget, TestHarnessParams::default())
    }

    /// Builds harness with given root widget and window size.
    pub fn create_with_size(
        default_props: DefaultProperties,
        root_widget: NewWidget<W>,
        window_size: Size,
    ) -> Self {
        Self::create_with(
            default_props,
            root_widget,
            TestHarnessParams {
                window_size,
                ..Default::default()
            },
        )
    }

    /// Builds harness with given root widget and additional parameters.
    pub fn create_with(
        default_props: DefaultProperties,
        root_widget: NewWidget<W>,
        params: TestHarnessParams,
    ) -> Self {
        let mouse_state = PointerState::default();
        // TODO - Change params.window_size type and remove this step
        #[allow(
            clippy::cast_possible_truncation,
            reason = "If sizes are large enough to overflow a u32, we have other problems"
        )]
        let window_size = PhysicalSize::new(
            params.window_size.width as _,
            params.window_size.height as _,
        );

        // If there is no default tracing subscriber, we set our own. If one has
        // already been set, we get an error which we swallow.
        // Having a default subscriber is helpful for tests; swallowing errors means
        // we don't panic if the user has already set one, or a test creates multiple
        // harnesses.
        let _ = try_init_test_tracing();

        const ROBOTO: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/fonts/roboto/Roboto-Regular.ttf"
        ));
        let data = Blob::new(Arc::new(ROBOTO));

        let (signal_sender, signal_receiver) = mpsc::channel::<RenderRootSignal>();

        let dummy_tree_update = TreeUpdate {
            nodes: vec![(0.into(), Node::new(Role::Window))],
            tree: Some(Tree {
                root: 0.into(),
                toolkit_name: None,
                toolkit_version: None,
            }),
            focus: 0.into(),
        };
        let mut harness = Self {
            signal_receiver,
            render_root: RenderRoot::new(
                root_widget,
                move |signal| signal_sender.send(signal).unwrap(),
                RenderRootOptions {
                    default_properties: Arc::new(default_props),
                    use_system_fonts: false,
                    size_policy: WindowSizePolicy::User,
                    scale_factor: params.scale_factor,
                    test_font: Some(data),
                },
            ),
            access_tree: accesskit_consumer::Tree::new(dummy_tree_update, false),
            render_context: None,
            vello_renderer: None,
            mouse_state,
            window_size,
            background_color: params.background_color,
            screenshot_tolerance: params.screenshot_tolerance,
            panic_on_rewrite_saturation: params.panic_on_rewrite_saturation,
            max_screenshot_size: params.max_screenshot_size,
            action_queue: VecDeque::new(),
            has_ime_session: false,
            ime_rect: Default::default(),
            clipboard: String::new(),
            title: String::new(),
            _marker: PhantomData,
        };

        // Set up the initial state, and clear invalidation flags.
        harness.process_window_event(WindowEvent::Resize(window_size));
        harness.process_window_event(WindowEvent::EnableAccessTree);
        harness.animate_ms(0);

        let (_, tree_update) = harness.render_root.redraw();
        let tree_update = tree_update.unwrap();
        harness
            .access_tree
            .update_and_process_changes(tree_update, &mut NoOpTreeChangeHandler);

        harness
    }

    // --- MARK: PROCESS EVENTS

    /// Send a [`WindowEvent`] to the simulated window.
    ///
    /// This will run [rewrite passes](masonry_core::doc::pass_system#rewrite-passes) after the event is processed.
    pub fn process_window_event(&mut self, event: WindowEvent) -> Handled {
        let handled = self.render_root.handle_window_event(event);
        self.process_signals();
        handled
    }

    /// Send a [`PointerEvent`] to the simulated window.
    ///
    /// This will run [rewrite passes](masonry_core::doc::pass_system#rewrite-passes) after the event is processed.
    pub fn process_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let handled = self.render_root.handle_pointer_event(event);
        self.process_signals();
        handled
    }

    /// Send a [`TextEvent`] to the simulated window.
    ///
    /// This will run [rewrite passes](masonry_core::doc::pass_system#rewrite-passes) after the event is processed.
    pub fn process_text_event(&mut self, event: TextEvent) -> Handled {
        let handled = self.render_root.handle_text_event(event);
        self.process_signals();
        handled
    }

    /// Send an [`ActionRequest`] to the simulated window.
    ///
    /// This will run [rewrite passes](masonry_core::doc::pass_system#rewrite-passes) after the event is processed.
    pub fn process_access_event(&mut self, event: ActionRequest) {
        self.render_root.handle_access_event(event);
        self.process_signals();
    }

    // This should be ran after any operation which runs the rewrite passes
    // (i.e. processing an event, etc.)
    fn process_signals(&mut self) {
        if self.panic_on_rewrite_saturation && self.render_root.needs_rewrite_passes() {
            panic!("Loop detected in rewrite passes");
        }
        while let Some(signal) = self.signal_receiver.try_iter().next() {
            match signal {
                RenderRootSignal::Action(action, widget_id) => {
                    self.action_queue.push_back((action, widget_id));
                }
                RenderRootSignal::StartIme => {
                    self.has_ime_session = true;
                }
                RenderRootSignal::EndIme => {
                    self.has_ime_session = false;
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    self.ime_rect = (position, size);
                }
                RenderRootSignal::ClipboardStore(text) => {
                    self.clipboard = text;
                }
                RenderRootSignal::RequestRedraw => (),
                RenderRootSignal::RequestAnimFrame => (),
                RenderRootSignal::TakeFocus => (),
                RenderRootSignal::SetCursor(_) => (),
                RenderRootSignal::SetSize(physical_size) => {
                    self.window_size = physical_size;
                    self.process_window_event(WindowEvent::Resize(physical_size));
                }
                RenderRootSignal::SetTitle(title) => {
                    self.title = title;
                }
                RenderRootSignal::DragWindow => (),
                RenderRootSignal::DragResizeWindow(_) => (),
                RenderRootSignal::ToggleMaximized => (),
                RenderRootSignal::Minimize => (),
                RenderRootSignal::Exit => (),
                RenderRootSignal::ShowWindowMenu(_) => (),
                RenderRootSignal::WidgetSelectedInInspector(_) => (),
            }
        }
    }

    // --- MARK: RENDER
    // TODO - We add way too many dependencies in this code
    // TODO - Should be async?
    /// Renders the window into an image and updates the `accesskit_consumer` tree.
    ///
    /// The returned image contains a bitmap (an array of pixels) as an 8-bits-per-channel RGB image.
    pub fn render(&mut self) -> RgbaImage {
        let (scene, tree_update) = self.render_root.redraw();
        let tree_update = tree_update.unwrap();
        self.access_tree
            .update_and_process_changes(tree_update, &mut NoOpTreeChangeHandler);
        if std::env::var("SKIP_RENDER_TESTS").is_ok_and(|it| !it.is_empty()) {
            return RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
        }

        let mut context = self
            .render_context
            .take()
            .unwrap_or_else(RenderContext::new);

        let device_id =
            pollster::block_on(context.device(None)).expect("No compatible device found");
        let device_handle = &mut context.devices[device_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        let mut renderer = self.vello_renderer.take().unwrap_or_else(|| {
            vello::Renderer::new(
                device,
                vello::RendererOptions {
                    // TODO - Examine this value
                    use_cpu: true,
                    num_init_threads: NonZeroUsize::new(1),
                    // TODO - Examine this value
                    antialiasing_support: vello::AaSupport::area_only(),
                    ..Default::default()
                },
            )
            .expect("Got non-Send/Sync error from creating renderer")
        });

        // TODO - fix window_size
        let (width, height) = (self.window_size.width, self.window_size.height);

        // Avoid having a zero-sized image
        let width = width.max(1);
        let height = height.max(1);

        let render_params = vello::RenderParams {
            base_color: self.background_color,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
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
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&TextureViewDescriptor::default());
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
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
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
        buf_slice.map_async(MapMode::Read, move |v| sender.send(v).unwrap());
        let recv_result = block_on_wgpu(device, receiver.receive()).expect("channel was closed");
        recv_result.expect("failed to map buffer");

        let data = buf_slice.get_mapped_range();
        let mut result_unpadded =
            Vec::<u8>::with_capacity((width * height * 4).try_into().unwrap());
        for row in 0..height {
            let start = (row * padded_byte_width).try_into().unwrap();
            result_unpadded.extend(&data[start..start + (width * 4) as usize]);
        }

        self.render_context = Some(context);
        self.vello_renderer = Some(renderer);

        RgbaImage::from_vec(width, height, result_unpadded).expect("failed to create image")
    }

    /// Get a reference to the current state of the accessibility tree.
    pub fn access_tree(&self) -> &accesskit_consumer::Tree {
        &self.access_tree
    }

    /// Get a reference to the current value of a node of the accessibility tree.
    pub fn access_node(&self, id: WidgetId) -> Option<accesskit_consumer::Node<'_>> {
        self.access_tree.state().node_by_id(id.into())
    }

    // --- MARK: EVENT HELPERS

    /// Move an internal mouse state, and send a [`Move`](PointerEvent::Move) event to the window.
    pub fn mouse_move(&mut self, pos: impl Into<Point>) {
        // FIXME - Account for scaling
        let Point { x, y } = pos.into();
        let pos = PhysicalPosition { x, y };
        self.mouse_state.position = pos;

        debug!("Harness mouse moved to {x}, {y}");

        self.process_pointer_event(PointerEvent::Move(PointerUpdate {
            pointer: PRIMARY_MOUSE,
            current: self.mouse_state.clone(),
            coalesced: vec![],
            predicted: vec![],
        }));
    }

    /// Send a [`Down`](PointerEvent::Down) event to the window.
    pub fn mouse_button_press(&mut self, button: PointerButton) {
        self.mouse_state.buttons.insert(button);
        self.process_pointer_event(PointerEvent::Down(PointerButtonEvent {
            pointer: PRIMARY_MOUSE,
            button: button.into(),
            state: self.mouse_state.clone(),
        }));
    }

    /// Send an [`Up`](PointerEvent::Up) event to the window.
    pub fn mouse_button_release(&mut self, button: PointerButton) {
        self.mouse_state.buttons.remove(button);
        self.process_pointer_event(PointerEvent::Down(PointerButtonEvent {
            pointer: PRIMARY_MOUSE,
            button: button.into(),
            state: self.mouse_state.clone(),
        }));
    }

    /// Send a [`Scroll`](PointerEvent::Scroll) event to the window.
    pub fn mouse_wheel(&mut self, Vec2 { x, y }: Vec2) {
        self.process_pointer_event(PointerEvent::Scroll(PointerScrollEvent {
            pointer: PRIMARY_MOUSE,
            delta: ScrollDelta::PixelDelta(PhysicalPosition { x, y }),
            state: self.mouse_state.clone(),
        }));
    }

    /// Send events that lead to a given widget being clicked.
    ///
    /// Combines [`mouse_move`](Self::mouse_move), [`mouse_button_press`](Self::mouse_button_press), and [`mouse_button_release`](Self::mouse_button_release).
    ///
    /// # Panics
    ///
    /// - If the widget is not found in the tree.
    /// - If the widget is stashed.
    /// - If the widget doesn't accept pointer events.
    /// - If the widget is scrolled out of view.
    #[track_caller]
    pub fn mouse_click_on(&mut self, id: WidgetId) {
        self.mouse_move_to(id);
        self.mouse_button_press(PointerButton::Primary);
        self.mouse_button_release(PointerButton::Primary);
    }

    /// Use [`mouse_move`](Self::mouse_move) to set the internal mouse pos to the center of the given widget.
    ///
    /// # Panics
    ///
    /// - If the widget is not found in the tree.
    /// - If the widget is stashed.
    /// - If the widget doesn't accept pointer events.
    /// - If the widget is scrolled out of view.
    #[track_caller]
    pub fn mouse_move_to(&mut self, id: WidgetId) {
        let widget = self.get_widget_with_id(id);
        let local_widget_center = (widget.ctx().size() / 2.0).to_vec2().to_point();
        let widget_center = widget.ctx().window_transform() * local_widget_center;

        if !widget.ctx().accepts_pointer_interaction() {
            panic!("Widget {id} doesn't accept pointer events");
        }
        if widget.ctx().is_stashed() {
            panic!("Widget {id} is stashed");
        }
        if self
            .render_root
            .get_root_widget()
            .find_widget_under_pointer(widget_center)
            .map(|w| w.id())
            != Some(id)
        {
            panic!("Widget {id} is not visible");
        }

        self.mouse_move(widget_center);
    }

    /// Try to get the target widget into the viewport.
    ///
    /// This will send an accesskit [`ScrollIntoView`] action to the widget,
    /// which will usually send [`RequestPanToChild`] events to the widget's parents.
    /// If the widget is hidden because it's "scrolled away", this should make it visible again.
    ///
    /// [`RequestPanToChild`]: masonry_core::core::Update::RequestPanToChild
    /// [`ScrollIntoView`]: masonry_core::accesskit::Action::ScrollIntoView
    #[track_caller]
    pub fn scroll_into_view(&mut self, id: WidgetId) {
        self.render_root.handle_access_event(ActionRequest {
            action: Action::ScrollIntoView,
            target: id.to_raw().into(),
            data: None,
        });
    }

    // TODO - Handle complicated IME
    // TODO - Mock Winit keyboard events
    /// Send a [`TextEvent`] for each character in the given string.
    pub fn keyboard_type_chars(&mut self, text: &str) {
        // For each character
        for c in text.split("").filter(|s| !s.is_empty()) {
            let event = TextEvent::Ime(Ime::Commit(c.to_string()));
            self.render_root.handle_text_event(event);
        }
        self.process_signals();
    }

    /// Send a [`TextEvent`] representing the user pressing the `Tab` key, either with or without the `Shift` key pressed.
    pub fn press_tab_key(&mut self, shift: bool) {
        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let event = TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Named(NamedKey::Tab),
            code: Code::Unidentified,
            modifiers,
            ..KeyboardEvent::default()
        });
        self.render_root.handle_text_event(event);
        self.process_signals();
    }

    /// Sets the [focused widget](masonry_core::doc::masonry_concepts#text-focus)
    /// and the [focus anchor](masonry_core::doc::masonry_concepts#focus-anchor).
    ///
    /// # Panics
    ///
    /// If the widget is not found in the tree or can't be focused.
    #[track_caller]
    pub fn focus_on(&mut self, id: Option<WidgetId>) {
        if let Some(id) = id {
            let Some(widget) = self.render_root.get_widget(id) else {
                panic!("Cannot focus widget {id}: widget not found in tree");
            };
            if widget.ctx().is_stashed() {
                panic!("Cannot focus widget {id}: widget is stashed");
            }
            if widget.ctx().is_disabled() {
                panic!("Cannot focus widget {id}: widget is disabled");
            }
        }
        let succeeded = self.render_root.focus_on(id);
        assert!(
            succeeded,
            "RenderRoot::focus_on refused a widget which TestHarness::focus_on accepted."
        );
        self.process_signals();
    }

    /// Sets the [focus fallback](masonry_core::doc::masonry_concepts#focus-fallback).
    pub fn set_focus_fallback(&mut self, id: Option<WidgetId>) {
        if let Some(id) = id {
            let Some(_) = self.render_root.get_widget(id) else {
                panic!("Cannot set widget {id} as focus fallback: widget not found in tree");
            };
        }
        let _ = self.render_root.set_focus_fallback(id);
    }

    /// Run an animation pass on the widget tree.
    pub fn animate_ms(&mut self, ms: u64) {
        self.render_root
            .handle_window_event(WindowEvent::AnimFrame(Duration::from_millis(ms)));
        self.process_signals();
    }

    /// Helper method to directly enable/disable a widget.
    pub fn set_disabled(&mut self, widget: WidgetTag<impl Widget>, disabled: bool) {
        self.edit_widget(widget, |mut target| {
            target.ctx.set_disabled(disabled);
        });
    }

    // --- MARK: GETTERS

    /// Return a [`WidgetRef`] to the root widget.
    pub fn root_widget(&self) -> WidgetRef<'_, W> {
        self.render_root.get_root_widget().downcast().unwrap()
    }

    /// Return the [`WidgetId`] of the root widget.
    pub fn root_id(&self) -> WidgetId {
        self.render_root.get_root_widget().id()
    }

    /// Return a [`WidgetRef`] to the widget with the given id.
    ///
    /// # Panics
    ///
    /// Panics if no widget with this id can be found.
    #[track_caller]
    pub fn get_widget_with_id(&self, id: WidgetId) -> WidgetRef<'_, dyn Widget> {
        self.render_root
            .get_widget(id)
            .unwrap_or_else(|| panic!("could not find widget {id}"))
    }

    /// Return a [`WidgetRef`] to the widget with the given tag.
    ///
    /// # Panics
    ///
    /// Panics if no widget with this tag can be found.
    #[track_caller]
    pub fn get_widget<W2: Widget + FromDynWidget + ?Sized>(
        &self,
        tag: WidgetTag<W2>,
    ) -> WidgetRef<'_, W2> {
        self.render_root
            .get_widget_with_tag(tag)
            .unwrap_or_else(|| panic!("could not find widget '{tag}'"))
    }

    /// Drain the events recorded by the [`Recorder`] widget with the given tag.
    ///
    /// # Panics
    ///
    /// Panics if no widget with this tag can be found.
    #[track_caller]
    pub fn take_records_of<W2: Widget>(&self, tag: WidgetTag<Recorder<W2>>) -> Vec<Record> {
        self.get_widget(tag).inner().recording().drain()
    }

    /// Flush the events recorded by the [`Recorder`] widget with the given tag.
    ///
    /// # Panics
    ///
    /// Panics if no widget with this tag can be found.
    #[track_caller]
    pub fn flush_records_of<W2: Widget>(&self, tag: WidgetTag<Recorder<W2>>) {
        self.get_widget(tag).inner().recording().clear();
    }

    /// Try to return a [`WidgetRef`] to the widget with the given id.
    pub fn try_get_widget(&self, id: WidgetId) -> Option<WidgetRef<'_, dyn Widget>> {
        self.render_root.get_widget(id)
    }

    /// Return a [`WidgetRef`] to the [focused widget](masonry_core::doc::masonry_concepts#text-focus).
    pub fn focused_widget(&self) -> Option<WidgetRef<'_, dyn Widget>> {
        self.render_root
            .get_root_widget()
            .find_widget_by_id(self.render_root.focused_widget()?)
    }

    /// Return the id of the [focused widget](masonry_core::doc::masonry_concepts#text-focus).
    pub fn focused_widget_id(&self) -> Option<WidgetId> {
        self.render_root.focused_widget()
    }

    /// Return a [`WidgetRef`] to the widget which [captures pointer events](masonry_core::doc::masonry_concepts#pointer-capture).
    pub fn pointer_capture_target(&self) -> Option<WidgetRef<'_, dyn Widget>> {
        self.render_root
            .get_widget(self.render_root.pointer_capture_target()?)
    }

    /// Return the id of the widget which [captures pointer events](masonry_core::doc::masonry_concepts#pointer-capture).
    // TODO - This is kinda redundant with the above
    pub fn pointer_capture_target_id(&self) -> Option<WidgetId> {
        self.render_root.pointer_capture_target()
    }

    /// Call the provided visitor on every widget in the widget tree.
    pub fn inspect_widgets(&mut self, mut f: impl FnMut(WidgetRef<'_, dyn Widget>)) {
        fn inspect(
            widget: WidgetRef<'_, dyn Widget>,
            f: &mut impl FnMut(WidgetRef<'_, dyn Widget>),
        ) {
            f(widget);
            for child in widget.children() {
                inspect(child, f);
            }
        }

        inspect(self.render_root.get_root_widget(), &mut f);
    }

    /// Get a [`WidgetMut`] to the root widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_root_widget<R>(&mut self, f: impl FnOnce(WidgetMut<'_, W>) -> R) -> R {
        let ret = self.render_root.edit_root_widget(|mut root| {
            let root = root.downcast::<W>();
            f(root)
        });
        self.process_signals();
        ret
    }

    /// Get a [`WidgetMut`] to a specific widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_widget_with_id<R>(
        &mut self,
        id: WidgetId,
        f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
    ) -> R {
        let ret = self.render_root.edit_widget(id, f);
        self.process_signals();
        ret
    }

    /// Get a [`WidgetMut`] to the widget with the given tag.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    #[track_caller]
    pub fn edit_widget<R, W2: Widget + FromDynWidget + ?Sized>(
        &mut self,
        tag: WidgetTag<W2>,
        f: impl FnOnce(WidgetMut<'_, W2>) -> R,
    ) -> R {
        let ret = self.render_root.edit_widget_with_tag(tag, f);
        self.process_signals();
        ret
    }

    /// Pop the oldest [`ErasedAction`] emitted by the widget tree, downcasting it to `T`.
    ///
    /// # Panics
    ///
    /// If there is an action, but it is not of type `T`.
    #[track_caller]
    pub fn pop_action<T: AnyDebug>(&mut self) -> Option<(T, WidgetId)> {
        let (action, widget) = self.pop_action_erased()?;
        let action = action.downcast().unwrap_or_else(|action| {
            panic!(
                "Expected Action to be of type {}, but got a value of type {} ({action:?}).",
                std::any::type_name::<T>(),
                (*action).type_name()
            )
        });
        Some((*action, widget))
    }

    /// Pop the oldest [`ErasedAction`] emitted by the widget tree.
    pub fn pop_action_erased(&mut self) -> Option<(ErasedAction, WidgetId)> {
        self.action_queue.pop_front()
    }

    /// Return the app's current cursor icon.
    ///
    /// The cursor icon is the icon that would be displayed to indicate the mouse
    /// position in a visual environment.
    pub fn cursor_icon(&self) -> CursorIcon {
        self.render_root.cursor_icon()
    }

    /// Return whether the app has an IME session in progress.
    ///
    /// This usually means that a widget which [accepts text input](Widget::accepts_text_input) is focused.
    pub fn has_ime_session(&self) -> bool {
        self.has_ime_session
    }

    /// Return the rectangle of the IME session.
    ///
    /// This is usually the layout rectangle of the focused widget.
    pub fn ime_rect(&self) -> (LogicalPosition<f64>, LogicalSize<f64>) {
        self.ime_rect
    }

    /// Returns the contents of the emulated clipboard.
    ///
    /// This is an empty string by default.
    pub fn clipboard_contents(&self) -> String {
        self.clipboard.clone()
    }

    /// Return the size of the simulated window.
    pub fn window_size(&self) -> PhysicalSize<u32> {
        self.window_size
    }

    /// Return the title of the simulated window.
    pub fn title(&self) -> std::string::String {
        self.title.clone()
    }

    // --- MARK: SNAPSHOT

    /// Method used by [`assert_render_snapshot`] and [`assert_failing_render_snapshot`]. Use these macros, not this method.
    ///
    /// Renders the current widget tree to a pixmap, and compares the pixmap against the
    /// snapshot stored in `<CRATE ROOT>/screenshots/<test_name>.png`.
    ///
    /// * `manifest_dir`: directory where `Cargo.toml` can be found.
    /// * `test_name`: arbitrary name; second argument of [`assert_render_snapshot`].
    /// * `expect_failure`: whether the snapshot is expected to fail to match.
    #[doc(hidden)]
    #[track_caller]
    pub fn check_render_snapshot(
        &mut self,
        manifest_dir: &str,
        test_name: &str,
        expect_failure: bool,
    ) {
        if std::env::var("SKIP_RENDER_TESTS").is_ok_and(|it| !it.is_empty()) {
            // We still redraw to get some coverage in the paint code.
            let _ = self.render_root.redraw();

            return;
        }
        let max_size = Some(usize::try_from(self.max_screenshot_size).unwrap());

        #[track_caller]
        fn save_image(image: &DynamicImage, path: &PathBuf, max_size: Option<usize>) {
            let mut buffer = Cursor::new(Vec::new());
            image.write_to(&mut buffer, ImageFormat::Png).unwrap();

            let image_data = buffer.into_inner();
            // Whenever we save a file, we optimise it.
            // This avoids cases where people copy the `new` file to the reference path, thus avoiding optimisation
            // (We could skip this for diff images, but that is so far off the hot path that it's not
            // worth a different code path for it.)
            let data =
                optimize_from_memory(image_data.as_slice(), &Options::from_preset(5)).unwrap();
            let saved_len = data.len();
            std::fs::write(path, data).unwrap();
            if let Some(max_size) = max_size
                && saved_len > max_size
            {
                panic!(
                    "New screenshot file ({saved_len} bytes) was larger than the supported file size ({max_size} bytes).\
                        Consider increasing `TestHarnessParams::max_screenshot_size` when creating the test harness.",
                );
            }
        }

        let new_image: DynamicImage = self.render().into();

        let screenshots_folder = PathBuf::from(manifest_dir).join("screenshots");
        std::fs::create_dir_all(&screenshots_folder).unwrap();

        let reference_path = screenshots_folder.join(format!("{test_name}.png"));
        let new_path = screenshots_folder.join(format!("{test_name}.new.png"));
        let diff_path = screenshots_folder.join(format!("{test_name}.diff.png"));

        let bless_test = std::env::var_os("MASONRY_TEST_BLESS").is_some_and(|it| !it.is_empty());

        let Ok(reference_file) = File::open(&reference_path) else {
            if bless_test && !expect_failure {
                let _ = std::fs::remove_file(&new_path);
                let _ = std::fs::remove_file(&diff_path);
                save_image(&new_image, &reference_path, max_size);
                return;
            }

            // Remove '<test_name>.new.png' file if it exists
            let _ = std::fs::remove_file(&new_path);
            save_image(&new_image, &new_path, max_size);
            panic!("Snapshot test '{test_name}' failed: No reference file");
        };
        let reference_size = reference_file.metadata().unwrap().len();

        let reference_file =
            ImageReader::with_format(BufReader::new(reference_file), ImageFormat::Png);

        // TODO: If this file is corrupted, it could be an lfs bandwidth/installation issue.
        // Have a warning for that case (i.e. differentiation between not-found and invalid format)
        // and a environment variable to ignore the test in that case.
        let ref_image = reference_file.decode().unwrap().to_rgb8();

        if expect_failure {
            if get_image_diff(&ref_image, &new_image.to_rgb8(), self.screenshot_tolerance).is_some()
            {
                return;
            } else {
                panic!(
                    "Snapshot test '{test_name}' did not fail as expected: Images are identical"
                );
            }
        }

        if let Some(diff_image) =
            get_image_diff(&ref_image, &new_image.to_rgb8(), self.screenshot_tolerance)
        {
            if bless_test {
                let _ = std::fs::remove_file(&new_path);
                let _ = std::fs::remove_file(&diff_path);
                save_image(&new_image, &reference_path, max_size);
            } else {
                save_image(&new_image, &new_path, max_size);
                // Don't fail if the diff file is too big!
                save_image(&diff_image.into(), &diff_path, None);
                panic!("Snapshot test '{test_name}' failed: Images are different");
            }
        } else {
            // Remove the vestigial new and diff images
            let _ = std::fs::remove_file(&new_path);
            let _ = std::fs::remove_file(&diff_path);
            if reference_size > u64::from(self.max_screenshot_size) {
                panic!(
                    "Existing file ({reference_size}) was larger than the supported file size ({}).\
                    Consider increasing `TestHarnessParams::max_screenshot_size` when creating the test harness.",
                    self.max_screenshot_size
                );
            }
        }
    }
}

struct NoOpTreeChangeHandler;

impl accesskit_consumer::TreeChangeHandler for NoOpTreeChangeHandler {
    fn node_added(&mut self, _node: &accesskit_consumer::Node<'_>) {}

    fn node_updated(
        &mut self,
        _old_node: &accesskit_consumer::Node<'_>,
        _new_node: &accesskit_consumer::Node<'_>,
    ) {
    }

    fn focus_moved(
        &mut self,
        _old_node: Option<&accesskit_consumer::Node<'_>>,
        _new_node: Option<&accesskit_consumer::Node<'_>>,
    ) {
    }

    fn node_removed(&mut self, _node: &accesskit_consumer::Node<'_>) {}
}
