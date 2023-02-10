// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

// FIXME
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, VecDeque};
use std::ops::DerefMut;
use std::rc::Rc;

use druid_shell::text::InputHandler;
// TODO - rename Application to AppHandle in glazier
// See https://github.com/linebender/glazier/issues/44
use druid_shell::{Application as AppHandle, WindowHandle};
use druid_shell::{
    Cursor, FileDialogToken, FileInfo, Region, TextFieldToken, TimerToken, WindowBuilder,
};
// Automatically defaults to std::time::Instant on non Wasm platforms
use instant::Instant;
use tracing::{error, info, info_span};

use crate::action::ActionQueue;
use crate::app_delegate::{AppDelegate, DelegateCtx, NullDelegate};
use crate::command::CommandQueue;
use crate::contexts::GlobalPassCtx;
use crate::debug_logger::DebugLogger;
use crate::ext_event::{ExtEventQueue, ExtEventSink, ExtMessage};
use crate::kurbo::{Point, Size};
use crate::piet::{Color, Piet, RenderContext};
use crate::platform::{
    DialogInfo, WindowConfig, WindowSizePolicy, EXT_EVENT_IDLE_TOKEN, RUN_COMMANDS_TOKEN,
};
use crate::testing::MockTimerQueue;
use crate::text::TextFieldRegistration;
use crate::widget::{FocusChange, StoreInWidgetMut, WidgetMut, WidgetRef, WidgetState};
use crate::{
    command as sys_cmd, ArcStr, BoxConstraints, Command, Env, Event, EventCtx, Handled,
    InternalEvent, InternalLifeCycle, LayoutCtx, LifeCycle, LifeCycleCtx, MasonryWinHandler,
    PaintCtx, PlatformError, Target, Widget, WidgetCtx, WidgetId, WidgetPod, WindowDescription,
    WindowId,
};

/// The type of a function that will be called once an IME field is updated.
pub type ImeUpdateFn = dyn FnOnce(druid_shell::text::Event);

// TODO - Add AppRootEvent type

// TODO - Explain and document re-entrancy and when locks should be used - See issue #16

// TODO - Delegate callbacks are shared between AppRoot and AppRootInner methods
// This muddles what part of the code has the responsibility of maintaining invariants

/// State shared by all windows in the UI.
///
/// This is an internal object that shouldn't be manipulated directly by the user.
#[derive(Clone)]
pub struct AppRoot {
    inner: Rc<RefCell<AppRootInner>>,
}

struct AppRootInner {
    app_handle: AppHandle,
    debug_logger: DebugLogger,
    app_delegate: Box<dyn AppDelegate>,
    command_queue: CommandQueue,
    action_queue: ActionQueue,
    ext_event_queue: ExtEventQueue,
    file_dialogs: HashMap<FileDialogToken, DialogInfo>,
    window_requests: VecDeque<WindowDescription>,
    pending_windows: HashMap<WindowId, PendingWindow>,
    active_windows: HashMap<WindowId, WindowRoot>,
    // FIXME - remove
    main_window_id: WindowId,
    /// The id of the most-recently-focused window that has a menu. On macOS, this
    /// is the window that's currently in charge of the app menu.
    #[allow(unused)]
    menu_window: Option<WindowId>,
    env: Env,
}

/// The parts of a window, pending construction, that are dependent on top level app state
/// or are not part of druid-shell's windowing abstraction.
struct PendingWindow {
    root: Box<dyn Widget>,
    title: ArcStr,
    transparent: bool,
    size_policy: WindowSizePolicy,
}

// TODO - refactor out again
/// Per-window state not owned by user code.
///
/// This is an internal object that shouldn't be manipulated directly by the user.
pub struct WindowRoot {
    pub(crate) id: WindowId,
    pub(crate) root: WidgetPod<Box<dyn Widget>>,
    pub(crate) title: ArcStr,
    size_policy: WindowSizePolicy,
    size: Size,
    invalid: Region,
    // Is `Some` if the most recently displayed frame was an animation frame.
    pub(crate) last_anim: Option<Instant>,
    pub(crate) last_mouse_pos: Option<Point>,
    pub(crate) focus: Option<WidgetId>,
    pub(crate) ext_event_sink: ExtEventSink,
    pub(crate) handle: WindowHandle,
    pub(crate) timers: HashMap<TimerToken, WidgetId>,
    // Used in unit tests - see `src/testing/mock_timer_queue.rs`
    pub(crate) mock_timer_queue: Option<MockTimerQueue>,
    pub(crate) transparent: bool,
    pub(crate) ime_handlers: Vec<(TextFieldToken, TextFieldRegistration)>,
    pub(crate) ime_focus_change: Option<Option<TextFieldToken>>,
}

// ---

// Public methods
//
// Each of these methods should handle post-event cleanup
// (eg invalidation regions, opening new windows, etc)
impl AppRoot {
    /// Create new application.
    pub(crate) fn create(
        app: AppHandle,
        windows: Vec<WindowDescription>,
        app_delegate: Option<Box<dyn AppDelegate>>,
        ext_event_queue: ExtEventQueue,
        env: Env,
    ) -> Result<Self, PlatformError> {
        let inner = Rc::new(RefCell::new(AppRootInner {
            app_handle: app,
            debug_logger: DebugLogger::new(false),
            app_delegate: app_delegate.unwrap_or_else(|| Box::new(NullDelegate)),
            command_queue: VecDeque::new(),
            action_queue: VecDeque::new(),
            ext_event_queue,
            file_dialogs: HashMap::new(),
            // FIXME - this is awful
            main_window_id: windows.first().unwrap().id,
            menu_window: None,
            env,
            window_requests: VecDeque::new(),
            pending_windows: Default::default(),
            active_windows: Default::default(),
        }));
        let mut app_root = AppRoot { inner };

        for desc in windows {
            let window = app_root.build_native_window(desc)?;
            window.show();
        }

        Ok(app_root)
    }

    /// Notify the app that a window was added and is now running in the platform.
    ///
    /// This should be called by the platform after processing from
    /// [`druid_shell::WindowBuilder`] finishes.
    pub fn window_connected(&mut self, window_id: WindowId, handle: WindowHandle) {
        {
            let mut inner = self.inner.borrow_mut();
            let inner = inner.deref_mut();

            if let Some(pending) = inner.pending_windows.remove(&window_id) {
                let win = WindowRoot::new(
                    window_id,
                    handle,
                    inner.ext_event_queue.make_sink(),
                    pending.root,
                    pending.title,
                    pending.transparent,
                    pending.size_policy,
                    None,
                );
                let existing = inner.active_windows.insert(window_id, win);
                debug_assert!(existing.is_none(), "duplicate window");
            } else {
                tracing::error!("no window for connecting handle {:?}", window_id);
            }

            // If the external event host has no handle, it cannot wake us
            // when an event arrives.
            if inner.ext_event_queue.handle_window_id.is_none() {
                inner.set_ext_event_idle_handler(window_id);
            }
        }

        self.with_delegate(|delegate, ctx, env| delegate.on_window_added(ctx, window_id, env));

        let event = Event::WindowConnected;
        self.do_window_event(window_id, event);

        self.process_commands_and_actions();
        self.inner().invalidate_paint_regions();
        self.process_ime_changes();
        self.process_window_requests();
    }

    /// Notify the app that a window has been closed by the platform.
    ///
    /// AppRoot then cleans up resources.
    pub fn window_removed(&mut self, window_id: WindowId) {
        self.with_delegate(|delegate, ctx, env| delegate.on_window_removed(ctx, window_id, env));

        let mut inner = self.inner.borrow_mut();
        inner.active_windows.remove(&window_id);

        // If there are no active or pending windows, we quit the run loop.
        if inner.active_windows.is_empty() && inner.pending_windows.is_empty() {
            #[cfg(any(target_os = "windows", feature = "x11"))]
            inner.app_handle.quit();
        }

        // If we are closing the window that is currently responsible
        // for waking us when external events arrive, we want to pass
        // that responsibility to another window.
        if inner.ext_event_queue.handle_window_id == Some(window_id) {
            inner.ext_event_queue.handle_window_id = None;
            // find any other live window
            let win_id = inner.active_windows.keys().next();
            if let Some(any_other_window) = win_id.cloned() {
                inner.set_ext_event_idle_handler(any_other_window);
            }
        }
    }

    /// Notify the app that a window has acquired focus (eg the user clicked on it).
    pub fn window_got_focus(&mut self, _window_id: WindowId) {
        // TODO - menu stuff
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns [`Handled::Yes`] if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on Windows)
    /// the OS needs to know if an event was handled.
    pub fn handle_event(&mut self, event: Event, window_id: WindowId) -> Handled {
        let result;
        {
            if let Event::Command(command)
            | Event::Internal(InternalEvent::TargetedCommand(command)) = event
            {
                self.do_cmd(command);
                result = Handled::Yes;
            } else {
                result = self.do_window_event(window_id, event);
            };
        }

        self.process_commands_and_actions();
        self.inner().invalidate_paint_regions();
        self.process_ime_changes();
        self.process_window_requests();

        result
    }

    /// Handle a 'command' message from druid-shell. These map to an item
    /// in an application, window, or context (right-click) menu.
    ///
    /// If the menu is associated with a window (the general case) then
    /// the `window_id` will be `Some(_)`, otherwise (such as if no window
    /// is open but a menu exists, as on macOS) it will be `None`.
    pub fn handle_system_cmd(&mut self, cmd_id: u32, window_id: Option<WindowId>) {
        #![allow(unused_variables)]
        todo!();
    }

    // TODO - Promises
    /// Notify the app that the user has closed a given dialog popup.
    ///
    /// This gives the user both a token referring to the given dialog and
    /// the [`FileInfo`] representing which file(s) the user chose.
    pub fn handle_dialog_response(&mut self, token: FileDialogToken, file_info: Option<FileInfo>) {
        let dialog_info = self.inner().file_dialogs.remove(&token);
        if let Some(dialog_info) = dialog_info {
            let cmd = if let Some(info) = file_info {
                dialog_info.accept_cmd.with(info).to(dialog_info.id)
            } else {
                dialog_info.cancel_cmd.to(dialog_info.id)
            };
            self.do_cmd(cmd);
            self.process_commands_and_actions();
            self.process_ime_changes();
            self.inner().invalidate_paint_regions();
        } else {
            tracing::error!("unknown dialog token");
        }
    }

    /// Run some computations before painting a given window.
    ///
    /// Must be called once per frame for each window.
    ///
    /// Currently, this computes layout and runs an animation frame.
    pub fn prepare_paint(&mut self, window_id: WindowId) {
        {
            let mut inner = self.inner.borrow_mut();
            let inner = inner.deref_mut();
            if let Some(win) = inner.active_windows.get_mut(&window_id) {
                win.prepare_paint(
                    &mut inner.debug_logger,
                    &mut inner.command_queue,
                    &mut inner.action_queue,
                    &inner.env,
                );
            }
            inner.invalidate_paint_regions();
        }
        self.process_window_requests();
    }

    /// Paint a given window's contents.
    ///
    /// Currently, this computes layout if needed and calls paint methods in the
    /// widget hierarchy.
    pub fn paint(&mut self, window_id: WindowId, piet: &mut Piet, invalid: &Region) {
        let mut inner = self.inner.borrow_mut();
        let inner = inner.deref_mut();
        if let Some(win) = inner.active_windows.get_mut(&window_id) {
            win.do_paint(
                piet,
                invalid,
                &mut inner.debug_logger,
                &mut inner.command_queue,
                &mut inner.action_queue,
                &inner.env,
            );
        }
    }

    /// Run any leftover commands from previous events.
    pub fn run_commands(&mut self) {
        self.process_commands_and_actions();
        self.inner().invalidate_paint_regions();
        self.process_ime_changes();
        self.process_window_requests();
    }

    /// Run any events in the background event queue, usually sent by a background thread.
    pub fn run_ext_events(&mut self) {
        self.process_ext_events();
        self.process_commands_and_actions();
        self.inner().invalidate_paint_regions();
        self.process_ime_changes();
        self.process_window_requests();
    }

    #[allow(missing_docs)]
    pub fn ime_update_fn(
        &self,
        window_id: WindowId,
        widget_id: WidgetId,
    ) -> Option<Box<ImeUpdateFn>> {
        let mut inner = self.inner.borrow_mut();
        let window = inner.active_windows.get(&window_id)?;
        window.ime_invalidation_fn(widget_id)
    }

    #[allow(missing_docs)]
    pub fn get_ime_lock(
        &mut self,
        window_id: WindowId,
        token: TextFieldToken,
        mutable: bool,
    ) -> Box<dyn InputHandler> {
        let mut inner = self.inner.borrow_mut();
        inner
            .active_windows
            .get_mut(&window_id)
            .unwrap()
            .get_ime_handler(token, mutable)
    }

    /// Returns a `WidgetId` if the lock was mutable; the widget should be updated.
    pub fn release_ime_lock(
        &mut self,
        window_id: WindowId,
        token: TextFieldToken,
    ) -> Option<WidgetId> {
        let mut inner = self.inner.borrow_mut();
        inner
            .active_windows
            .get_mut(&window_id)
            .unwrap()
            .release_ime_lock(token)
    }
}

// Internal functions
impl AppRoot {
    fn inner(&self) -> RefMut<'_, AppRootInner> {
        self.inner.borrow_mut()
    }

    // TODO - rename?
    fn process_commands_and_actions(&mut self) {
        loop {
            let next_cmd = self.inner().command_queue.pop_front();
            if let Some(cmd) = next_cmd {
                self.do_cmd(cmd);
                continue;
            }

            let next_action = self.inner().action_queue.pop_front();
            if let Some((action, widget_id, window_id)) = next_action {
                self.with_delegate(|delegate, ctx, env| {
                    delegate.on_action(ctx, window_id, widget_id, action, env)
                });
                continue;
            }

            // else - no more commands or actions
            break;
        }
    }

    fn process_ext_events(&mut self) {
        loop {
            let ext_cmd = self.inner().ext_event_queue.recv();
            match ext_cmd {
                Some(ExtMessage::Command(selector, payload, target)) => {
                    self.do_cmd(Command::from_ext(selector, payload, target))
                }
                Some(ExtMessage::Promise(promise_result, widget_id, window_id)) => {
                    // TODO
                    self.do_window_event(
                        window_id,
                        Event::Internal(InternalEvent::RoutePromiseResult(
                            promise_result,
                            widget_id,
                        )),
                    );
                }
                None => break,
            }
        }
    }

    fn process_ime_changes(&mut self) {
        let mut ime_focus_change_fns: Vec<Box<dyn Fn()>> = vec![];

        for window in self.inner().active_windows.values_mut() {
            if let Some(focus_change) = window.ime_focus_change.take() {
                // The handle.set_focused_text_field method may call WindowHandle
                // methods which may be reentrant (depending on the platform).
                // So we clone the window handle and defer calling set_focused_text_field.
                let handle = window.handle.clone();
                let f = Box::new(move || handle.set_focused_text_field(focus_change));
                ime_focus_change_fns.push(f);
            }
        }

        for ime_focus_change_fn in ime_focus_change_fns {
            (ime_focus_change_fn)()
        }
    }

    /// Handle a command. Top level commands (e.g. for creating and destroying
    /// windows) have their logic here; other commands are passed to the window.
    fn do_cmd(&mut self, cmd: Command) {
        if self.with_delegate(|delegate, ctx, env| delegate.on_command(ctx, &cmd, env))
            == Handled::Yes
        {
            return;
        }

        use Target as T;
        match cmd.target() {
            // these are handled the same no matter where they come from
            _ if cmd.is(sys_cmd::QUIT_APP) => self.inner().app_handle.quit(),
            #[cfg(target_os = "macos")]
            _ if cmd.is(sys_cmd::HIDE_APPLICATION) => self.inner().hide_app(),
            #[cfg(target_os = "macos")]
            _ if cmd.is(sys_cmd::HIDE_OTHERS) => self.inner().hide_others(),
            _ if cmd.is(sys_cmd::NEW_WINDOW) => {
                self.inner().request_new_window(cmd);
            }
            _ if cmd.is(sys_cmd::CLOSE_ALL_WINDOWS) => self.inner().request_close_all_windows(),
            //T::Window(id) if cmd.is(sys_cmd::INVALIDATE_IME) => self.inner().invalidate_ime(cmd, id),
            // these should come from a window
            // FIXME: we need to be able to open a file without a window handle
            // TODO - uncomment
            //T::Window(id) if cmd.is(sys_cmd::SHOW_OPEN_PANEL) => self.inner().show_open_panel(cmd, id),
            //T::Window(id) if cmd.is(sys_cmd::SHOW_SAVE_PANEL) => self.inner().show_save_panel(cmd, id),
            //T::Window(id) if cmd.is(sys_cmd::CONFIGURE_WINDOW) => self.inner().request_configure_window(cmd, id),
            T::Window(id) if cmd.is(sys_cmd::CLOSE_WINDOW) => {
                self.inner().request_close_window(id);
            }
            T::Window(id) if cmd.is(sys_cmd::SHOW_WINDOW) => self.inner().request_show_window(id),
            //T::Window(id) if cmd.is(sys_cmd::PASTE) => self.inner().do_paste(id),
            _ if cmd.is(sys_cmd::CLOSE_WINDOW) => {
                tracing::warn!("CLOSE_WINDOW command must target a window.")
            }
            _ if cmd.is(sys_cmd::SHOW_WINDOW) => {
                tracing::warn!("SHOW_WINDOW command must target a window.")
            }
            // TODO - uncomment
            /*
            _ if cmd.is(sys_cmd::SHOW_OPEN_PANEL) => {
                tracing::warn!("SHOW_OPEN_PANEL command must target a window.")
            }
            */
            _ => {
                self.inner().dispatch_cmd(cmd);
            }
        }
    }

    fn do_window_event(&mut self, source_id: WindowId, event: Event) -> Handled {
        if matches!(
            event,
            Event::Command(..) | Event::Internal(InternalEvent::TargetedCommand(..))
        ) {
            unreachable!("commands should be dispatched via dispatch_cmd");
        }

        if self.with_delegate(|delegate, ctx, env| delegate.on_event(ctx, source_id, &event, env))
            == Handled::Yes
        {
            return Handled::Yes;
        }

        let mut inner = self.inner.borrow_mut();
        let inner = inner.deref_mut();

        if let Some(win) = inner.active_windows.get_mut(&source_id) {
            win.event(
                event,
                &mut inner.debug_logger,
                &mut inner.command_queue,
                &mut inner.action_queue,
                &inner.env,
            )
        } else {
            // TODO - error message?
            Handled::No
        }
    }

    /// A helper fn for setting up the `DelegateCtx`. Takes a closure with
    /// an arbitrary return type `R`, and returns `Some(R)` if an `AppDelegate`
    /// is configured.
    fn with_delegate<R>(
        &mut self,
        f: impl FnOnce(&mut dyn AppDelegate, &mut DelegateCtx, &Env) -> R,
    ) -> R {
        let mut inner = self.inner.borrow_mut();
        let inner = inner.deref_mut();

        let mut window = inner.active_windows.get_mut(&inner.main_window_id).unwrap();
        let mut fake_widget_state;
        let res = {
            let mut global_state = GlobalPassCtx::new(
                window.ext_event_sink.clone(),
                &mut inner.debug_logger,
                &mut inner.command_queue,
                &mut inner.action_queue,
                &mut window.timers,
                window.mock_timer_queue.as_mut(),
                &window.handle,
                inner.main_window_id,
                window.focus,
            );
            fake_widget_state = window.root.state.clone();

            let main_root_ctx = WidgetCtx {
                global_state: &mut global_state,
                widget_state: &mut window.root.state,
            };
            let main_root_widget = WidgetMut {
                parent_widget_state: &mut fake_widget_state,
                inner: Box::from_widget_and_ctx(&mut window.root.inner, main_root_ctx),
            };

            let mut ctx = DelegateCtx {
                //command_queue: &mut inner.command_queue,
                ext_event_queue: &mut inner.ext_event_queue,
                main_root_widget,
            };

            f(&mut *inner.app_delegate, &mut ctx, &inner.env)
        };

        // TODO - handle cursor and validation

        window.post_event_processing(
            &mut fake_widget_state,
            &mut inner.debug_logger,
            &mut inner.command_queue,
            &mut inner.action_queue,
            &inner.env,
            false,
        );

        res
    }

    // -- Handle "new window" requests --

    fn process_window_requests(&mut self) {
        let window_requests = std::mem::take(&mut self.inner.borrow_mut().window_requests);
        for window_desc in window_requests.into_iter() {
            match self.build_native_window(window_desc) {
                Ok(window) => window.show(),
                Err(err) => tracing::error!("failed to create window: '{err}'"),
            };
        }
    }

    // TODO - document why process_window_requests/build_native_window
    fn build_native_window(
        &mut self,
        desc: WindowDescription,
    ) -> Result<WindowHandle, crate::PlatformError> {
        let root = desc.root;
        let title = desc.title;
        let config = desc.config;
        let id = desc.id;

        let mut builder = WindowBuilder::new(self.inner.borrow().app_handle.clone());
        config.apply_to_builder(&mut builder);
        builder.set_title(title.to_string());

        let handler = MasonryWinHandler::new_shared(self.clone(), id);
        builder.set_handler(Box::new(handler));

        let pending = PendingWindow {
            root,
            title,
            transparent: config.transparent.unwrap_or(false),
            size_policy: config.size_policy,
        };

        let existing = self.inner.borrow_mut().pending_windows.insert(id, pending);
        assert!(existing.is_none(), "duplicate pending window {id:?}");

        builder.build()
    }
}

impl AppRootInner {
    /// invalidate any window handles that need it.
    ///
    /// This should always be called at the end of an event update cycle,
    /// including for lifecycle events.
    fn invalidate_paint_regions(&mut self) {
        for win in self.active_windows.values_mut() {
            win.invalidate_paint_region();
        }
    }

    /// Set the idle handle that will be used to wake us when external events arrive.
    fn set_ext_event_idle_handler(&mut self, id: WindowId) {
        if let Some(mut idle) = self
            .active_windows
            .get_mut(&id)
            .and_then(|win| win.handle.get_idle_handle())
        {
            if self.ext_event_queue.has_pending_items() {
                idle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
            }
            self.ext_event_queue.set_idle(idle, id);
        }
    }

    fn request_new_window(&mut self, cmd: Command) {
        let desc = cmd.get(sys_cmd::NEW_WINDOW);
        // The NEW_WINDOW command is private and only masonry should be able to send it,
        // so we can use .unwrap() here
        let desc = *desc
            .take()
            .unwrap()
            .downcast::<WindowDescription>()
            .unwrap();
        self.window_requests.push_back(desc);
    }

    /// triggered by a menu item or other command.
    ///
    /// This doesn't close the window; it calls the close method on the platform
    /// window handle; the platform should close the window, and then call
    /// our handlers `destroy()` method, at which point we can do our cleanup.
    fn request_close_window(&mut self, window_id: WindowId) {
        if let Some(window) = self.active_windows.get_mut(&window_id) {
            let handled = window.event(
                Event::WindowCloseRequested,
                &mut self.debug_logger,
                &mut self.command_queue,
                &mut self.action_queue,
                &self.env,
            );
            if !handled.is_handled() {
                window.event(
                    Event::WindowDisconnected,
                    &mut self.debug_logger,
                    &mut self.command_queue,
                    &mut self.action_queue,
                    &self.env,
                );
                window.handle.close();
            }
        } else {
            tracing::warn!("Failed to close {window_id:?}: no active window with this id");
        }
    }

    // TODO - same confirmation code as request_close_window?
    /// Requests the platform to close all windows.
    fn request_close_all_windows(&mut self) {
        for win in self.active_windows.values_mut() {
            win.handle.close();
        }
    }

    fn request_show_window(&mut self, id: WindowId) {
        if let Some(win) = self.active_windows.get_mut(&id) {
            win.handle.bring_to_front_and_focus();
        }
    }

    fn request_configure_window(&mut self, config: &WindowConfig, id: WindowId) {
        if let Some(win) = self.active_windows.get_mut(&id) {
            config.apply_to_handle(&mut win.handle);
        }
    }

    fn dispatch_cmd(&mut self, cmd: Command) -> Handled {
        self.invalidate_paint_regions();
        match cmd.target() {
            Target::Global => {
                for w in self.active_windows.values_mut() {
                    if w.event(
                        Event::Command(cmd.clone()),
                        &mut self.debug_logger,
                        &mut self.command_queue,
                        &mut self.action_queue,
                        &self.env,
                    )
                    .is_handled()
                    {
                        return Handled::Yes;
                    }
                }
                return Handled::No;
            }
            Target::Window(id) => {
                if let Some(w) = self.active_windows.get_mut(&id) {
                    return w.event(
                        Event::Command(cmd),
                        &mut self.debug_logger,
                        &mut self.command_queue,
                        &mut self.action_queue,
                        &self.env,
                    );
                }
            }
            // in this case we send it to every window that might contain
            // this widget, breaking if the event is handled.
            Target::Widget(id) => {
                for w in self
                    .active_windows
                    .values_mut()
                    .filter(|w| w.may_contain_widget(id))
                {
                    let event = Event::Internal(InternalEvent::TargetedCommand(cmd.clone()));
                    if w.event(
                        event,
                        &mut self.debug_logger,
                        &mut self.command_queue,
                        &mut self.action_queue,
                        &self.env,
                    )
                    .is_handled()
                    {
                        return Handled::Yes;
                    }
                }
            }
            Target::Auto => {
                tracing::error!("{:?} reached window handler with `Target::Auto`", cmd);
            }
        }
        Handled::No
    }

    #[cfg(FALSE)]
    fn show_open_panel(&mut self, cmd: Command, window_id: WindowId) {
        let options = cmd.get(sys_cmd::SHOW_OPEN_PANEL).to_owned();
        let handle = self
            .inner
            .borrow_mut()
            .active_windows
            .get_mut(&window_id)
            .map(|w| w.handle.clone());

        let accept_cmd = options.accept_cmd.unwrap_or(sys_cmd::OPEN_FILE);
        let cancel_cmd = options.cancel_cmd.unwrap_or(sys_cmd::OPEN_PANEL_CANCELLED);
        let token = handle.and_then(|mut handle| handle.open_file(options.opt));
        if let Some(token) = token {
            self.root().file_dialogs.insert(
                token,
                DialogInfo {
                    id: window_id,
                    accept_cmd,
                    cancel_cmd,
                },
            );
        }
    }

    #[cfg(FALSE)]
    fn show_save_panel(&mut self, cmd: Command, window_id: WindowId) {
        let options = cmd.get(sys_cmd::SHOW_SAVE_PANEL).to_owned();
        let handle = self
            .inner
            .borrow_mut()
            .active_windows
            .get_mut(&window_id)
            .map(|w| w.handle.clone());
        let accept_cmd = options.accept_cmd.unwrap_or(sys_cmd::SAVE_FILE_AS);
        let cancel_cmd = options.cancel_cmd.unwrap_or(sys_cmd::SAVE_PANEL_CANCELLED);
        let token = handle.and_then(|mut handle| handle.save_as(options.opt));
        if let Some(token) = token {
            self.root().file_dialogs.insert(
                token,
                DialogInfo {
                    id: window_id,
                    accept_cmd,
                    cancel_cmd,
                },
            );
        }
    }

    #[cfg(target_os = "macos")]
    fn hide_app(&self) {
        use druid_shell::platform::mac::ApplicationExt as _;
        self.app_handle.hide();
    }

    #[cfg(target_os = "macos")]
    fn hide_others(&mut self) {
        use druid_shell::platform::mac::ApplicationExt as _;
        self.app_handle.hide_others();
    }
}

// ---

impl WindowRoot {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: WindowId,
        handle: WindowHandle,
        ext_event_sink: ExtEventSink,
        root: Box<dyn Widget>,
        title: ArcStr,
        transparent: bool,
        size_policy: WindowSizePolicy,
        mock_timer_queue: Option<MockTimerQueue>,
    ) -> WindowRoot {
        WindowRoot {
            id,
            root: WidgetPod::new(root),
            size_policy,
            size: Size::ZERO,
            invalid: Region::EMPTY,
            title,
            transparent,
            last_anim: None,
            last_mouse_pos: None,
            focus: None,
            ext_event_sink,
            handle,
            timers: HashMap::new(),
            mock_timer_queue,
            ime_handlers: Vec::new(),
            ime_focus_change: None,
        }
    }

    // TODO - Add 'get_global_ctx() -> GlobalPassCtx' method

    /// `true` iff any child requested an animation frame since the last `AnimFrame` event.
    pub(crate) fn wants_animation_frame(&self) -> bool {
        self.root.state().request_anim
    }

    pub(crate) fn focus_chain(&self) -> &[WidgetId] {
        &self.root.state().focus_chain
    }

    /// Returns `true` if the provided widget may be in this window,
    /// but it may also be a false positive.
    /// However when this returns `false` the widget is definitely not in this window.
    pub(crate) fn may_contain_widget(&self, widget_id: WidgetId) -> bool {
        // The bloom filter we're checking can return false positives.
        widget_id == self.root.id() || self.root.state().children.may_contain(&widget_id)
    }

    pub(crate) fn post_event_processing(
        &mut self,
        widget_state: &mut WidgetState,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
        process_commands: bool,
    ) {
        // If children are changed during the handling of an event,
        // we need to send RouteWidgetAdded now, so that they are ready for update/layout.
        if widget_state.children_changed {
            // Anytime widgets are removed we check and see if any of those
            // widgets had IME sessions and unregister them if so.
            let WindowRoot {
                ime_handlers,
                handle,
                ..
            } = self;
            ime_handlers.retain(|(token, v)| {
                let will_retain = v.is_alive();
                if !will_retain {
                    tracing::debug!("{:?} removed", token);
                    handle.remove_text_field(*token);
                }
                will_retain
            });

            self.lifecycle(
                &LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded),
                debug_logger,
                command_queue,
                action_queue,
                env,
                false,
            );
        }

        if debug_logger.layout_tree.root.is_none() {
            debug_logger.layout_tree.root = Some(self.root.id().to_raw() as u32);
        }

        if self.root.state().needs_window_origin && !self.root.state().needs_layout {
            let event = LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin);
            self.lifecycle(
                &event,
                debug_logger,
                command_queue,
                action_queue,
                env,
                false,
            );
        }

        // Update the disabled state if necessary
        // Always do this before updating the focus-chain
        if self.root.state().tree_disabled_changed() {
            let event = LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged);
            self.lifecycle(
                &event,
                debug_logger,
                command_queue,
                action_queue,
                env,
                false,
            );
        }

        // Update the focus-chain if necessary
        // Always do this before sending focus change, since this event updates the focus chain.
        if self.root.state().update_focus_chain {
            let event = LifeCycle::BuildFocusChain;
            self.lifecycle(
                &event,
                debug_logger,
                command_queue,
                action_queue,
                env,
                false,
            );
        }

        self.update_focus(widget_state, debug_logger, command_queue, action_queue, env);

        // If we need a new paint pass, make sure druid-shell knows it.
        if self.wants_animation_frame() {
            self.handle.request_anim_frame();
        }
        self.invalid.union_with(&widget_state.invalid);
        for ime_field in widget_state.text_registrations.drain(..) {
            let token = self.handle.add_text_field();
            tracing::debug!("{:?} added", token);
            self.ime_handlers.push((token, ime_field));
        }

        // If there are any commands and they should be processed
        if process_commands && !command_queue.is_empty() {
            // Ask the handler to call us back on idle
            // so we can process them in a new event/update pass.
            if let Some(mut handle) = self.handle.get_idle_handle() {
                handle.schedule_idle(RUN_COMMANDS_TOKEN);
            } else {
                // FIXME - probably messes with tests
                error!("failed to get idle handle");
            }
        }
    }

    pub(crate) fn event(
        &mut self,
        event: Event,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) -> Handled {
        match &event {
            Event::WindowSize(size) => self.size = *size,
            Event::MouseDown(e) | Event::MouseUp(e) | Event::MouseMove(e) | Event::Wheel(e) => {
                self.last_mouse_pos = Some(e.pos)
            }
            Event::Internal(InternalEvent::MouseLeave) => self.last_mouse_pos = None,
            _ => (),
        }

        let event = match event {
            Event::Timer(token) => {
                if let Some(widget_id) = self.timers.get(&token) {
                    Event::Internal(InternalEvent::RouteTimer(token, *widget_id))
                } else {
                    error!("No widget found for timer {:?}", token);
                    return Handled::No;
                }
            }
            other => other,
        };

        if let Event::WindowConnected = event {
            self.lifecycle(
                &LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded),
                debug_logger,
                command_queue,
                action_queue,
                env,
                false,
            );
        }

        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size), "<root>");
        let is_handled = {
            let mut global_state = GlobalPassCtx::new(
                self.ext_event_sink.clone(),
                debug_logger,
                command_queue,
                action_queue,
                &mut self.timers,
                self.mock_timer_queue.as_mut(),
                &self.handle,
                self.id,
                self.focus,
            );
            let mut notifications = VecDeque::new();

            let mut ctx = EventCtx {
                global_state: &mut global_state,
                widget_state: &mut widget_state,
                notifications: &mut notifications,
                is_handled: false,
                is_root: true,
                request_pan_to_child: None,
            };

            {
                ctx.global_state
                    .debug_logger
                    .push_important_span(&format!("EVENT {}", event.short_name()));
                let _span = info_span!("event").entered();
                self.root.on_event(&mut ctx, &event, env);
                ctx.global_state.debug_logger.pop_span();
            }

            if !ctx.notifications.is_empty() {
                info!("{} unhandled notifications:", ctx.notifications.len());
                for (i, n) in ctx.notifications.iter().enumerate() {
                    info!("{}: {:?}", i, n);
                }
            }

            Handled::from(ctx.is_handled)
        };

        // Clean up the timer token and do it immediately after the event handling
        // because the token may be reused and re-added in a lifecycle pass below.
        if let Event::Internal(InternalEvent::RouteTimer(token, _)) = event {
            self.timers.remove(&token);
        }

        if let Some(cursor) = &widget_state.cursor {
            self.handle.set_cursor(cursor);
        } else if matches!(
            event,
            Event::MouseMove(..) | Event::Internal(InternalEvent::MouseLeave)
        ) {
            self.handle.set_cursor(&Cursor::Arrow);
        }

        if matches!(
            (event, self.size_policy),
            (Event::WindowSize(_), WindowSizePolicy::Content)
        ) {
            // Because our initial size can be zero, the window system won't ask us to paint.
            // So layout ourselves and hopefully we resize
            self.layout(debug_logger, command_queue, action_queue, env);
        }

        self.post_event_processing(
            &mut widget_state,
            debug_logger,
            command_queue,
            action_queue,
            env,
            false,
        );

        self.root.as_dyn().debug_validate(false);

        is_handled
    }

    pub(crate) fn lifecycle(
        &mut self,
        event: &LifeCycle,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
        process_commands: bool,
    ) {
        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size), "<root>");
        let mut global_state = GlobalPassCtx::new(
            self.ext_event_sink.clone(),
            debug_logger,
            command_queue,
            action_queue,
            &mut self.timers,
            self.mock_timer_queue.as_mut(),
            &self.handle,
            self.id,
            self.focus,
        );
        let mut ctx = LifeCycleCtx {
            global_state: &mut global_state,
            widget_state: &mut widget_state,
        };

        {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("LIFECYCLE {}", event.short_name()));
            let _span = info_span!("lifecycle").entered();
            self.root.lifecycle(&mut ctx, event, env);
            ctx.global_state.debug_logger.pop_span();
        }

        self.post_event_processing(
            &mut widget_state,
            debug_logger,
            command_queue,
            action_queue,
            env,
            process_commands,
        );
    }

    pub(crate) fn invalidate_paint_region(&mut self) {
        if self.root.state().needs_layout {
            // TODO - this might be too coarse
            self.handle.invalidate();
        } else {
            for rect in self.invalid.rects() {
                self.handle.invalidate_rect(*rect);
            }
        }
        self.invalid.clear();
    }

    #[allow(dead_code)]
    pub(crate) fn invalid(&self) -> &Region {
        &self.invalid
    }

    #[allow(dead_code)]
    pub(crate) fn invalid_mut(&mut self) -> &mut Region {
        &mut self.invalid
    }

    /// Get ready for painting, by doing layout and sending an `AnimFrame` event.
    pub(crate) fn prepare_paint(
        &mut self,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) {
        let now = Instant::now();
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/linebender/druid/issues/85 for discussion.
        let last = self.last_anim.take();
        let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

        if self.wants_animation_frame() {
            self.event(
                Event::AnimFrame(elapsed_ns),
                debug_logger,
                command_queue,
                action_queue,
                env,
            );
            self.last_anim = Some(now);
        }
    }

    pub(crate) fn do_paint(
        &mut self,
        piet: &mut Piet,
        invalid: &Region,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) {
        if self.root.state().needs_layout {
            self.layout(debug_logger, command_queue, action_queue, env);
        }

        for &r in invalid.rects() {
            piet.clear(
                Some(r),
                if self.transparent {
                    Color::TRANSPARENT
                } else {
                    env.get(crate::theme::WINDOW_BACKGROUND_COLOR)
                },
            );
        }
        self.paint(
            piet,
            invalid,
            debug_logger,
            command_queue,
            action_queue,
            env,
        );
    }

    pub(crate) fn layout(
        &mut self,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) {
        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size), "<root>");
        let mut global_state = GlobalPassCtx::new(
            self.ext_event_sink.clone(),
            debug_logger,
            command_queue,
            action_queue,
            &mut self.timers,
            self.mock_timer_queue.as_mut(),
            &self.handle,
            self.id,
            self.focus,
        );
        let mut layout_ctx = LayoutCtx {
            global_state: &mut global_state,
            widget_state: &mut widget_state,
            mouse_pos: self.last_mouse_pos,
        };
        let bc = match self.size_policy {
            WindowSizePolicy::User => BoxConstraints::tight(self.size),
            WindowSizePolicy::Content => BoxConstraints::UNBOUNDED,
        };

        let content_size = {
            layout_ctx
                .global_state
                .debug_logger
                .push_important_span("LAYOUT");
            let _span = info_span!("layout").entered();
            self.root.layout(&mut layout_ctx, &bc, env)
        };
        layout_ctx.global_state.debug_logger.pop_span();

        if let WindowSizePolicy::Content = self.size_policy {
            let insets = self.handle.content_insets();
            let full_size = (content_size.to_rect() + insets).size();
            if self.size != full_size {
                self.size = full_size;
                self.handle.set_size(full_size)
            }
        }
        layout_ctx.place_child(&mut self.root, Point::ORIGIN, env);
        self.lifecycle(
            &LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin),
            debug_logger,
            command_queue,
            action_queue,
            env,
            false,
        );
        self.post_event_processing(
            &mut widget_state,
            debug_logger,
            command_queue,
            action_queue,
            env,
            true,
        );
    }

    fn paint(
        &mut self,
        piet: &mut Piet,
        invalid: &Region,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) {
        let widget_state = WidgetState::new(self.root.id(), Some(self.size), "<root>");
        let mut global_state = GlobalPassCtx::new(
            self.ext_event_sink.clone(),
            debug_logger,
            command_queue,
            action_queue,
            &mut self.timers,
            self.mock_timer_queue.as_mut(),
            &self.handle,
            self.id,
            self.focus,
        );
        let mut ctx = PaintCtx {
            render_ctx: piet,
            global_state: &mut global_state,
            widget_state: &widget_state,
            z_ops: Vec::new(),
            region: invalid.clone(),
            depth: 0,
        };

        let root = &mut self.root;
        info_span!("paint").in_scope(|| {
            ctx.with_child_ctx(invalid.clone(), |ctx| root.paint_raw(ctx, env));
        });

        let mut z_ops = std::mem::take(&mut ctx.z_ops);
        z_ops.sort_by_key(|k| k.z_index);

        for z_op in z_ops.into_iter() {
            ctx.with_child_ctx(invalid.clone(), |ctx| {
                ctx.with_save(|ctx| {
                    ctx.render_ctx.transform(z_op.transform);
                    (z_op.paint_func)(ctx);
                });
            });
        }

        if self.wants_animation_frame() {
            self.handle.request_anim_frame();
        }
    }

    pub(crate) fn get_ime_handler(
        &mut self,
        req_token: TextFieldToken,
        mutable: bool,
    ) -> Box<dyn InputHandler> {
        self.ime_handlers
            .iter()
            .find(|(token, _)| req_token == *token)
            .and_then(|(_, reg)| reg.document.acquire(mutable))
            .unwrap()
    }

    pub(crate) fn get_focused_ime_handler(
        &mut self,
        mutable: bool,
    ) -> Option<Box<dyn InputHandler>> {
        let focused_widget_id = self.focus?;
        self.ime_handlers
            .iter()
            .find(|(_, reg)| reg.widget_id == focused_widget_id)
            .and_then(|(_, reg)| reg.document.acquire(mutable))
    }

    fn update_focus(
        &mut self,
        widget_state: &mut WidgetState,
        debug_logger: &mut DebugLogger,
        command_queue: &mut CommandQueue,
        action_queue: &mut ActionQueue,
        env: &Env,
    ) {
        if let Some(focus_req) = widget_state.request_focus.take() {
            let old = self.focus;
            let new = self.widget_for_focus_request(focus_req);

            // TODO
            // Skip change if requested widget is disabled

            // Only send RouteFocusChanged in case there's actual change
            if old != new {
                let event = LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { old, new });
                self.lifecycle(
                    &event,
                    debug_logger,
                    command_queue,
                    action_queue,
                    env,
                    false,
                );
                self.focus = new;
                // check if the newly focused widget has an IME session, and
                // notify the system if so.
                //
                // If you're here because a profiler sent you: I guess I should've
                // used a hashmap?
                let old_was_ime = old
                    .map(|old| {
                        self.ime_handlers
                            .iter()
                            .any(|(_, sesh)| sesh.widget_id == old)
                    })
                    .unwrap_or(false);
                let maybe_active_text_field = self
                    .ime_handlers
                    .iter()
                    .find(|(_, sesh)| Some(sesh.widget_id) == self.focus)
                    .map(|(token, _)| *token);
                // we call this on every focus change; we could call it less but does it matter?
                self.ime_focus_change = if maybe_active_text_field.is_some() {
                    Some(maybe_active_text_field)
                } else if old_was_ime {
                    Some(None)
                } else {
                    None
                };
            }
        }
    }

    /// Create a function that can invalidate the provided widget's text state.
    ///
    /// This will be called from outside the main app state in order to avoid
    /// reentrancy problems.
    pub(crate) fn ime_invalidation_fn(&self, widget: WidgetId) -> Option<Box<ImeUpdateFn>> {
        let token = self
            .ime_handlers
            .iter()
            .find(|(_, reg)| reg.widget_id == widget)
            .map(|(t, _)| *t)?;
        let window_handle = self.handle.clone();
        Some(Box::new(move |event| {
            window_handle.update_text_field(token, event)
        }))
    }

    /// Release a lock on an IME session, returning a `WidgetId` if the lock was mutable.
    ///
    /// After a mutable lock is released, the widget needs to be notified so that
    /// it can update any internal state.
    pub(crate) fn release_ime_lock(&mut self, req_token: TextFieldToken) -> Option<WidgetId> {
        let (_, reg) = self
            .ime_handlers
            .iter()
            .find(|(token, _)| req_token == *token)?;
        reg.document.release().then_some(reg.widget_id)
    }

    pub(crate) fn release_focused_ime_handler(&mut self) -> Option<WidgetId> {
        let focused_widget_id = self.focus?;
        let (_, reg) = self
            .ime_handlers
            .iter()
            .find(|(_, reg)| reg.widget_id == focused_widget_id)?;
        reg.document.release().then_some(reg.widget_id)
    }

    fn widget_for_focus_request(&self, focus: FocusChange) -> Option<WidgetId> {
        match focus {
            FocusChange::Resign => None,
            FocusChange::Focus(id) => Some(id),
            FocusChange::Next => self.widget_from_focus_chain(true),
            FocusChange::Previous => self.widget_from_focus_chain(false),
        }
    }

    fn widget_from_focus_chain(&self, forward: bool) -> Option<WidgetId> {
        self.focus.and_then(|focus| {
            self.focus_chain()
                .iter()
                // Find where the focused widget is in the focus chain
                .position(|id| id == &focus)
                .map(|idx| {
                    // Return the id that's next to it in the focus chain
                    let len = self.focus_chain().len();
                    let new_idx = if forward {
                        (idx + 1) % len
                    } else {
                        (idx + len - 1) % len
                    };
                    self.focus_chain()[new_idx]
                })
                .or_else(|| {
                    // If the currently focused widget isn't in the focus chain,
                    // then we'll just return the first/last entry of the chain, if any.
                    if forward {
                        self.focus_chain().first().copied()
                    } else {
                        self.focus_chain().last().copied()
                    }
                })
        })
    }

    /// Return the root widget.
    pub fn root_widget(&self, window_id: WindowId) -> WidgetRef<dyn Widget> {
        self.root.as_dyn()
    }

    /// Try to return the widget with the given id.
    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<WidgetRef<'_, dyn Widget>> {
        self.root.as_dyn().find_widget_by_id(id)
    }

    /// Recursively find innermost widget at given position.
    pub fn find_widget_at_pos(&self, pos: Point) -> Option<WidgetRef<'_, dyn Widget>> {
        self.root.as_dyn().find_widget_at_pos(pos)
    }

    /// Return the widget that receives keyboard events.
    pub fn focused_widget(&self) -> Option<WidgetRef<'_, dyn Widget>> {
        self.find_widget_by_id(self.focus?)
    }
}
