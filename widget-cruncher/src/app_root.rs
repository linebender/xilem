
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::kurbo::Size;
use crate::piet::Piet;

use crate::core::CommandQueue;
use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::window::{ImeUpdateFn, Window};
use crate::{
    Command, Data, Env, Event, Handled, InternalEvent, KeyEvent, PlatformError, Selector, Target,
    TimerToken, WidgetId, WindowDesc, WindowId,
};
use crate::win_handler::{DialogInfo, EXT_EVENT_IDLE_TOKEN};

use crate::window_handling::window_description::{PendingWindow, WindowConfig};
use crate::command::sys as sys_cmd;

use druid_shell::WindowBuilder;
use druid_shell::{
    text::InputHandler, Application, FileDialogToken, FileInfo, IdleToken, MouseEvent, Region,
    Scale, TextFieldToken, WinHandler, WindowHandle,
};

pub(crate) struct AppRoot {
    pub app: Application,
    pub command_queue: CommandQueue,
    pub file_dialogs: HashMap<FileDialogToken, DialogInfo>,
    pub ext_event_host: ExtEventHost,
    pub windows: Windows,
    /// The id of the most-recently-focused window that has a menu. On macOS, this
    /// is the window that's currently in charge of the app menu.
    #[allow(unused)]
    pub menu_window: Option<WindowId>,
    pub(crate) env: Env,
    pub ime_focus_change: Option<Box<dyn Fn()>>,
}

// TODO - remove
/// All active windows.
#[derive(Default)]
pub struct Windows {
    pub pending: HashMap<WindowId, PendingWindow>,
    pub active_windows: HashMap<WindowId, Window>,
}

impl Windows {
    pub fn connect(&mut self, id: WindowId, handle: WindowHandle, ext_handle: ExtEventSink) {
        if let Some(pending) = self.pending.remove(&id) {
            let win = Window::new(id, handle, pending, ext_handle);
            assert!(self.active_windows.insert(id, win).is_none(), "duplicate window");
        } else {
            tracing::error!("no window for connecting handle {:?}", id);
        }
    }

    pub fn add(&mut self, id: WindowId, win: PendingWindow) {
        assert!(self.pending.insert(id, win).is_none(), "duplicate pending");
    }

    pub fn count(&self) -> usize {
        self.active_windows.len() + self.pending.len()
    }
}


impl AppRoot {
    pub fn append_command(&mut self, cmd: Command) {
        self.command_queue.push_back(cmd);
    }

    pub fn connect(&mut self, id: WindowId, handle: WindowHandle) {
        self.windows
            .connect(id, handle, self.ext_event_host.make_sink());

        // If the external event host has no handle, it cannot wake us
        // when an event arrives.
        if self.ext_event_host.handle_window_id.is_none() {
            self.set_ext_event_idle_handler(id);
        }
    }

    /// Called after this window has been closed by the platform.
    ///
    /// We clean up resources.
    pub fn remove_window(&mut self, window_id: WindowId) {
        // when closing the last window:
        if let Some(mut win) = self.windows.active_windows.remove(&window_id) {
            if self.windows.active_windows.is_empty() {
                // If there are even no pending windows, we quit the run loop.
                if self.windows.count() == 0 {
                    #[cfg(any(target_os = "windows", feature = "x11"))]
                    self.app.quit();
                }
            }
        }

        // if we are closing the window that is currently responsible for
        // waking us when external events arrive, we want to pass that responsibility
        // to another window.
        if self.ext_event_host.handle_window_id == Some(window_id) {
            self.ext_event_host.handle_window_id = None;
            // find any other live window
            let win_id = self.windows.active_windows.keys().find(|k| *k != &window_id);
            if let Some(any_other_window) = win_id.cloned() {
                self.set_ext_event_idle_handler(any_other_window);
            }
        }
    }

    /// Set the idle handle that will be used to wake us when external events arrive.
    pub fn set_ext_event_idle_handler(&mut self, id: WindowId) {
        if let Some(mut idle) = self
            .windows
            .active_windows.get_mut(&id)
            .and_then(|win| win.handle.get_idle_handle())
        {
            if self.ext_event_host.has_pending_items() {
                idle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
            }
            self.ext_event_host.set_idle(idle, id);
        }
    }

    /// triggered by a menu item or other command.
    ///
    /// This doesn't close the window; it calls the close method on the platform
    /// window handle; the platform should close the window, and then call
    /// our handlers `destroy()` method, at which point we can do our cleanup.
    pub fn request_close_window(&mut self, window_id: WindowId) {
        if let Some(win) = self.windows.active_windows.get_mut(&window_id) {
            win.handle.close();
        }
    }

    /// Requests the platform to close all windows.
    pub fn request_close_all_windows(&mut self) {
        for win in self.windows.active_windows.values_mut() {
            win.handle.close();
        }
    }

    pub fn show_window(&mut self, id: WindowId) {
        if let Some(win) = self.windows.active_windows.get_mut(&id) {
            win.handle.bring_to_front_and_focus();
        }
    }

    pub fn configure_window(&mut self, config: &WindowConfig, id: WindowId) {
        if let Some(win) = self.windows.active_windows.get_mut(&id) {
            config.apply_to_handle(&mut win.handle);
        }
    }

    pub fn prepare_paint(&mut self, window_id: WindowId) {
        if let Some(win) = self.windows.active_windows.get_mut(&window_id) {
            win.prepare_paint(&mut self.command_queue, &self.env);
        }
        //self.do_update();
        self.invalidate_and_finalize();
    }

    pub fn paint(&mut self, window_id: WindowId, piet: &mut Piet, invalid: &Region) {
        if let Some(win) = self.windows.active_windows.get_mut(&window_id) {
            win.do_paint(
                piet,
                invalid,
                &mut self.command_queue,
                &self.env,
            );
        }
    }

    pub fn dispatch_cmd(&mut self, cmd: Command) -> Handled {
        self.invalidate_and_finalize();

        match cmd.target() {
            Target::Window(id) => {
                if let Some(w) = self.windows.active_windows.get_mut(&id) {
                    return if cmd.is(sys_cmd::CLOSE_WINDOW) {
                        let handled = w.event(
                            &mut self.command_queue,
                            Event::WindowCloseRequested,
                            &self.env,
                        );
                        if !handled.is_handled() {
                            w.event(
                                &mut self.command_queue,
                                Event::WindowDisconnected,
                                    &self.env,
                            );
                        }
                        handled
                    } else {
                        w.event(
                            &mut self.command_queue,
                            Event::Command(cmd),
                            &self.env,
                        )
                    };
                }
            }
            // in this case we send it to every window that might contain
            // this widget, breaking if the event is handled.
            Target::Widget(id) => {
                for w in self.windows.active_windows.values_mut().filter(|w| w.may_contain_widget(id)) {
                    let event = Event::Internal(InternalEvent::TargetedCommand(cmd.clone()));
                    if w.event(&mut self.command_queue, event, &self.env)
                        .is_handled()
                    {
                        return Handled::Yes;
                    }
                }
            }
            Target::Global => {
                for w in self.windows.active_windows.values_mut() {
                    let event = Event::Command(cmd.clone());
                    if w.event(&mut self.command_queue, event, &self.env)
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

    pub fn do_window_event(&mut self, source_id: WindowId, event: Event) -> Handled {
        match event {
            Event::Command(..) | Event::Internal(InternalEvent::TargetedCommand(..)) => {
                panic!("commands should be dispatched via dispatch_cmd");
            }
            _ => (),
        }

        if let Some(win) = self.windows.active_windows.get_mut(&source_id) {
            win.event(&mut self.command_queue, event, &self.env)
        } else {
            Handled::No
        }
    }

    pub fn do_update(&mut self) {
        /*
        // we send `update` to all windows, not just the active one:
        for window in self.windows.active_windows.values_mut() {
            window.update(&mut self.command_queue, &self.env);
            if let Some(focus_change) = window.ime_focus_change.take() {
                // we need to call this outside of the borrow, so we create a
                // closure that takes the correct window handle. yes, it feels
                // weird.
                let handle = window.handle.clone();
                let f = Box::new(move || handle.set_focused_text_field(focus_change));
                self.ime_focus_change = Some(f);
            }
        }
        */
        self.invalidate_and_finalize();
    }

    /// invalidate any window handles that need it.
    ///
    /// This should always be called at the end of an event update cycle,
    /// including for lifecycle events.
    pub fn invalidate_and_finalize(&mut self) {
        for win in self.windows.active_windows.values_mut() {
            win.invalidate_and_finalize();
        }
    }

    pub fn ime_update_fn(&self, window_id: WindowId, widget_id: WidgetId) -> Option<Box<ImeUpdateFn>> {
        self.windows
            .active_windows.get(&window_id)
            .and_then(|window| window.ime_invalidation_fn(widget_id))
    }

    pub fn get_ime_lock(
        &mut self,
        window_id: WindowId,
        token: TextFieldToken,
        mutable: bool,
    ) -> Box<dyn InputHandler> {
        self.windows
            .active_windows.get_mut(&window_id)
            .unwrap()
            .get_ime_handler(token, mutable)
    }

    /// Returns a `WidgetId` if the lock was mutable; the widget should be updated.
    pub fn release_ime_lock(&mut self, window_id: WindowId, token: TextFieldToken) -> Option<WidgetId> {
        self.windows
            .active_windows.get_mut(&window_id)
            .unwrap()
            .release_ime_lock(token)
    }

    pub fn window_got_focus(&mut self, _window_id: WindowId) {
        // TODO
    }
}
