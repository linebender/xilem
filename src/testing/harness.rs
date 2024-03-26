// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Tools and infrastructure for testing widgets.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use druid_shell::{KeyEvent, Modifiers, MouseButton, MouseButtons};
pub use druid_shell::{RawMods, Region};
use image::io::Reader as ImageReader;
use instant::Duration;
use shell::text::Selection;

use super::screenshots::{get_image_diff, get_rgba_image};
use super::snapshot_utils::get_cargo_workspace;
use super::MockTimerQueue;
use crate::action::{Action, ActionQueue};
//use crate::ext_event::ExtEventHost;
use crate::command::CommandQueue;
use crate::contexts::GlobalPassCtx;
use crate::debug_logger::DebugLogger;
use crate::ext_event::ExtEventQueue;
use crate::piet::{BitmapTarget, Device, ImageFormat, Piet};
use crate::widget::{StoreInWidgetMut, WidgetMut, WidgetRef};
use crate::*;

/// Default screen size for tests.
pub const HARNESS_DEFAULT_SIZE: Size = Size::new(400., 400.);

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
/// (Lifecycle and layout passes are handled automatically.)
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
///         Some((Action::ButtonPressed, button_id))
///     );
/// }
///
/// # simple_button();
/// ```
// TODO - Fix examples
pub struct TestHarness {
    mock_app: MockAppRoot,
    mouse_state: MouseEvent,
    window_size: Size,
}

/// Assert a snapshot of a rendered frame of your app.
///
/// This macro takes a test harness and a name, renders the current state of the app,
/// and stores the render as a PNG next to the text, in a `./screenshots/` folder.
///
/// If a screenshot already exists, the rendered value is compared against this screenshot.
/// The assert passes if both are equal; otherwise, a diff file is created.
///
/// If a screeshot doesn't exist, the assert will fail; the new screenshot is stored as
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

// TODO - merge
/// All of the state except for the `Piet` (render context). We need to pass
/// that in to get around some lifetime issues.
struct MockAppRoot {
    window: WindowRoot,
    command_queue: CommandQueue,
    action_queue: ActionQueue,
    debug_logger: DebugLogger,
}

impl TestHarness {
    /// Builds harness with given root widget.
    ///
    /// Window size will be [`HARNESS_DEFAULT_SIZE`].
    pub fn create(root: impl Widget) -> Self {
        Self::create_with_size(root, HARNESS_DEFAULT_SIZE)
    }

    /// Builds harness with given root widget and window size.
    pub fn create_with_size(root: impl Widget, window_size: Size) -> Self {
        //let ext_host = ExtEventHost::default();
        //let ext_handle = ext_host.make_sink();

        // FIXME
        let event_queue = ExtEventQueue::new();

        let window = WindowRoot::new(
            WindowId::next(),
            Default::default(),
            event_queue.make_sink(),
            Box::new(root),
            "Masonry test app".into(),
            false,
            WindowSizePolicy::User,
            Some(MockTimerQueue::new()),
        );

        let mouse_state = MouseEvent {
            pos: Point::ZERO,
            window_pos: Point::ZERO,
            buttons: MouseButtons::default(),
            mods: Modifiers::default(),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };

        let mut harness = TestHarness {
            mock_app: MockAppRoot {
                window,
                command_queue: VecDeque::new(),
                action_queue: VecDeque::new(),
                debug_logger: DebugLogger::new(false),
            },
            mouse_state,
            window_size,
        };

        // verify that all widgets are marked as having children_changed
        // (this should always be true for a new widget)
        harness.inspect_widgets(|widget| assert!(widget.state().children_changed));

        harness.process_event(Event::WindowConnected);
        harness.process_event(Event::WindowSize(window_size));

        harness
    }

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. Commands created as a result of this event
    /// will also be dispatched.
    pub fn process_event(&mut self, event: Event) {
        self.mock_app.event(event);

        self.process_state_after_event();
    }

    fn process_state_after_event(&mut self) {
        loop {
            let cmd = self.mock_app.command_queue.pop_front();
            match cmd {
                Some(cmd) => self
                    .mock_app
                    .event(Event::Internal(InternalEvent::TargetedCommand(cmd))),
                None => break,
            };
        }

        // TODO - this might be too coarse
        if self.root_widget().state().needs_layout {
            self.mock_app.layout();
            *self.window_mut().invalid_mut() = Region::from(self.window_size.to_rect());
        }
    }

    fn render_to(&mut self, render_target: &mut BitmapTarget) {
        /// A way to clean up resources when our render context goes out of
        /// scope, even during a panic.
        pub struct RenderContextGuard<'a>(Piet<'a>);

        impl Drop for RenderContextGuard<'_> {
            fn drop(&mut self) {
                // We need to call finish even if a test assert failed
                if let Err(err) = self.0.finish() {
                    // We can't panic, because we might already be panicking
                    tracing::error!("piet finish failed: {}", err);
                }
            }
        }

        let mut piet = RenderContextGuard(render_target.render_context());

        // FIXME - this doesn't make sense given we might render to a fresh surface
        let invalid = std::mem::replace(self.window_mut().invalid_mut(), Region::EMPTY);
        self.mock_app.paint_region(&mut piet.0, &invalid);
    }

    /// Create a Piet bitmap render context (an array of pixels), paint the
    /// window and return the bitmap.
    pub fn render(&mut self) -> Arc<[u8]> {
        let mut device = Device::new().expect("harness failed to get device");
        let mut render_target = device
            .bitmap_target(
                self.window_size.width as usize,
                self.window_size.height as usize,
                1.0,
            )
            .expect("failed to create bitmap_target");

        self.render_to(&mut render_target);

        render_target
            .to_image_buf(ImageFormat::RgbaPremul)
            .unwrap()
            .raw_pixels_shared()
    }

    // --- Event helpers ---

    /// Move an internal mouse state, and send a MouseMove event to the window.
    pub fn mouse_move(&mut self, pos: impl Into<Point>) {
        let pos = pos.into();
        // FIXME - not actually the same
        self.mouse_state.pos = pos;
        self.mouse_state.window_pos = pos;
        self.mouse_state.button = MouseButton::None;

        self.process_event(Event::MouseMove(self.mouse_state.clone()));
    }

    /// Send a MouseDown event to the window.
    pub fn mouse_button_press(&mut self, button: MouseButton) {
        self.mouse_state.buttons.insert(button);
        self.mouse_state.button = button;

        self.process_event(Event::MouseDown(self.mouse_state.clone()));
    }

    /// Send a MouseUp event to the window.
    pub fn mouse_button_release(&mut self, button: MouseButton) {
        self.mouse_state.buttons.remove(button);
        self.mouse_state.button = button;

        self.process_event(Event::MouseUp(self.mouse_state.clone()));
    }

    /// Send a Wheel event to the window
    pub fn mouse_wheel(&mut self, wheel_delta: Vec2) {
        self.mouse_state.button = MouseButton::None;
        self.mouse_state.wheel_delta = wheel_delta;

        self.process_event(Event::Wheel(self.mouse_state.clone()));
        self.mouse_state.wheel_delta = Vec2::ZERO;
    }

    /// Send events that lead to a given widget being clicked.
    ///
    /// Combines [`mouse_move`](Self::mouse_move), [`mouse_button_press`](Self::mouse_button_press), and [`mouse_button_release`](Self::mouse_button_release).
    pub fn mouse_click_on(&mut self, id: WidgetId) {
        let widget_rect = self.get_widget(id).state().window_layout_rect();
        let widget_center = widget_rect.center();

        self.mouse_move(widget_center);
        self.mouse_button_press(MouseButton::Left);
        self.mouse_button_release(MouseButton::Left);
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

    /// Simulate typing the given text.
    ///
    /// For every character in the input string (more specifically,
    /// for every Unicode Scalar Value), this sends a KeyDown and a
    /// KeyUp event to the window.
    ///
    /// Obviously this works better with ASCII text.
    ///
    /// **(Note: IME mocking is a future feature)**
    pub fn keyboard_type_chars(&mut self, text: &str) {
        // For each character
        for c in text.split("").filter(|s| !s.is_empty()) {
            let event = KeyEvent::for_test(RawMods::None, c);

            if self.mock_app.event(Event::KeyDown(event.clone())) == Handled::No {
                if let Some(mut input_handler) = self.mock_app.window.get_focused_ime_handler(true)
                {
                    // This is copy-pasted from druid-shell's simulate_input function
                    let selection = input_handler.selection();
                    input_handler.replace_range(selection.range(), c);
                    let new_caret_index = selection.min() + c.len();
                    input_handler.set_selection(Selection::caret(new_caret_index));

                    let modified_widget = self.mock_app.window.release_focused_ime_handler();

                    if let Some(widget_id) = modified_widget {
                        let event = Event::Internal(InternalEvent::RouteImeStateChange(widget_id));
                        self.mock_app.event(event);
                    }
                }
            }
            self.mock_app.event(Event::KeyUp(event.clone()));
        }
        self.process_state_after_event();
    }

    #[doc(alias = "send_command")]
    /// Send a command to a target.
    pub fn submit_command(&mut self, command: impl Into<Command>) {
        let command = command.into().default_to(self.mock_app.window.id.into());
        let event = Event::Internal(InternalEvent::TargetedCommand(command));
        self.process_event(event);
    }

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

    // --- Getters ---

    /// Return the mocked window.
    pub fn window(&self) -> &WindowRoot {
        &self.mock_app.window
    }

    /// Return the mocked window.
    pub fn window_mut(&mut self) -> &mut WindowRoot {
        &mut self.mock_app.window
    }

    /// Return the root widget.
    pub fn root_widget(&self) -> WidgetRef<'_, dyn Widget> {
        self.mock_app.window.root.as_dyn()
    }

    /// Return the widget with the given id.
    ///
    /// ## Panics
    ///
    /// Panics if no Widget with this id can be found.
    pub fn get_widget(&self, id: WidgetId) -> WidgetRef<'_, dyn Widget> {
        self.mock_app
            .window
            .find_widget_by_id(id)
            .expect("could not find widget")
    }

    /// Try to return the widget with the given id.
    pub fn try_get_widget(&self, id: WidgetId) -> Option<WidgetRef<'_, dyn Widget>> {
        self.mock_app.window.find_widget_by_id(id)
    }

    // TODO - link to focus documentation.
    /// Return the widget that receives keyboard events.
    pub fn focused_widget(&self) -> Option<WidgetRef<'_, dyn Widget>> {
        self.mock_app.window.focused_widget()
    }

    /// Call the provided visitor on every widget in the widget tree.
    pub fn inspect_widgets(&mut self, f: impl Fn(WidgetRef<'_, dyn Widget>) + 'static) {
        fn inspect(
            widget: WidgetRef<'_, dyn Widget>,
            f: &(impl Fn(WidgetRef<'_, dyn Widget>) + 'static),
        ) {
            f(widget);
            for child in widget.deref().children() {
                inspect(child, f);
            }
        }

        inspect(self.mock_app.window.root.as_dyn(), &f);
    }

    /// Get a [`WidgetMut`] to the root widget.
    ///
    /// Because of how WidgetMut works, it can only be passed to a user-provided callback.
    pub fn edit_root_widget<R>(
        &mut self,
        f: impl FnOnce(WidgetMut<'_, '_, Box<dyn Widget>>) -> R,
    ) -> R {
        // TODO - Move to MockAppRoot?
        let window = &mut self.mock_app.window;
        let mut fake_widget_state;
        let mut timers = HashMap::new();
        let res = {
            let mut global_state = GlobalPassCtx::new(
                window.ext_event_sink.clone(),
                &mut self.mock_app.debug_logger,
                &mut self.mock_app.command_queue,
                &mut self.mock_app.action_queue,
                &mut timers,
                window.mock_timer_queue.as_mut(),
                &window.handle,
                window.id,
                window.focus,
            );
            fake_widget_state = window.root.state.clone();

            let root_widget = WidgetMut {
                inner: Box::<dyn Widget>::from_widget_and_ctx(
                    &mut window.root.inner,
                    WidgetCtx {
                        global_state: &mut global_state,
                        widget_state: &mut window.root.state,
                    },
                ),
                parent_widget_state: &mut fake_widget_state,
            };

            f(root_widget)
        };

        // Timer creation should use mock_timer_queue instead
        assert!(timers.is_empty());

        // TODO - handle cursor and validation

        window.post_event_processing(
            &mut fake_widget_state,
            &mut self.mock_app.debug_logger,
            &mut self.mock_app.command_queue,
            &mut self.mock_app.action_queue,
            false,
        );
        self.process_state_after_event();

        res
    }

    /// Pop next action from the queue
    ///
    /// Note: Actions are still a WIP feature.
    pub fn pop_action(&mut self) -> Option<(Action, WidgetId)> {
        let (action, widget_id, _) = self.mock_app.action_queue.pop_front()?;
        Some((action, widget_id))
    }

    // --- Screenshots ---

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
            return;
        }

        let mut device = Device::new().expect("harness failed to get device");
        let mut render_target = device
            .bitmap_target(
                self.window_size.width as usize,
                self.window_size.height as usize,
                1.0,
            )
            .expect("failed to create bitmap_target");

        self.render_to(&mut render_target);

        let new_image = get_rgba_image(&mut render_target, self.window_size);

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

    // TODO - remove, see ROADMAP.md
    #[allow(missing_docs)]
    pub fn push_log(&mut self, message: &str) {
        self.mock_app
            .debug_logger
            .update_widget_state(self.mock_app.window.root.as_dyn());
        self.mock_app.debug_logger.push_log(false, message);
    }

    // ex: harness.write_debug_logs("test_log.json");
    #[allow(missing_docs)]
    pub fn write_debug_logs(&mut self, path: &str) {
        self.mock_app.debug_logger.write_to_file(path);
    }
}

#[allow(dead_code)]
impl MockAppRoot {
    fn event(&mut self, event: Event) -> Handled {
        self.window.event(
            event,
            &mut self.debug_logger,
            &mut self.command_queue,
            &mut self.action_queue,
        )
    }

    fn lifecycle(&mut self, event: LifeCycle) {
        self.window.lifecycle(
            &event,
            &mut self.debug_logger,
            &mut self.command_queue,
            &mut self.action_queue,
            false,
        );
    }

    fn layout(&mut self) {
        self.window.layout(
            &mut self.debug_logger,
            &mut self.command_queue,
            &mut self.action_queue,
        );
    }

    fn paint_region(&mut self, piet: &mut Piet, invalid: &Region) {
        self.window.do_paint(
            piet,
            invalid,
            &mut self.debug_logger,
            &mut self.command_queue,
            &mut self.action_queue,
        );
    }
}
