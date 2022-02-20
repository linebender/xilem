// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The implementation of the WinHandler trait (druid-shell integration).

use std::any::Any;

use crate::kurbo::Size;
use crate::piet::Piet;

use crate::command as sys_cmd;
use crate::{Event, InternalEvent, Selector, WindowId};

use crate::app_root::AppRoot;

use druid_shell::{
    text::InputHandler, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent, Region, Scale,
    TextFieldToken, TimerToken, WindowHandle,
};
use druid_shell::{AppHandler, WinHandler};

pub(crate) const RUN_COMMANDS_TOKEN: IdleToken = IdleToken::new(1);

/// A token we are called back with if an external event was submitted.
pub(crate) const EXT_EVENT_IDLE_TOKEN: IdleToken = IdleToken::new(2);

/// The struct implements the druid-shell `WinHandler` trait.
///
/// One `DruidHandler` exists per window.
///
/// This is something of an internal detail and possibly we don't want to surface
/// it publicly.
pub struct DruidWinHandler {
    /// The shared app state.
    pub(crate) app_state: AppRoot,
    /// The id for the current window.
    window_id: WindowId,
}

/// The top level event handler.
///
/// This corresponds to the `AppHandler` trait in druid-shell, which is only
/// used to handle events that are not associated with a window.
///
/// Currently, this means only menu items on macOS when no window is open.
pub struct DruidAppHandler {
    app_state: AppRoot,
}

// TODO - Move to separate file
/// The information for forwarding druid-shell's file dialog reply to the right place.
pub struct DialogInfo {
    /// The window to send the command to.
    pub id: WindowId,
    /// The command to send if the dialog is accepted.
    pub accept_cmd: Selector<FileInfo>,
    /// The command to send if the dialog is cancelled.
    pub cancel_cmd: Selector<()>,
}

impl DruidAppHandler {
    pub(crate) fn new(app_state: AppRoot) -> Self {
        Self { app_state }
    }
}

impl DruidWinHandler {
    /// Note: the root widget doesn't go in here, because it gets added to the
    /// app state.
    pub(crate) fn new_shared(app_state: AppRoot, window_id: WindowId) -> DruidWinHandler {
        DruidWinHandler {
            app_state,
            window_id,
        }
    }
}

impl AppHandler for DruidAppHandler {
    fn command(&mut self, id: u32) {
        self.app_state.handle_system_cmd(id, None)
    }
}

impl WinHandler for DruidWinHandler {
    fn connect(&mut self, handle: &WindowHandle) {
        self.app_state
            .window_connected(self.window_id, handle.clone());
    }

    fn request_close(&mut self) {
        let event = Event::Command(sys_cmd::CLOSE_WINDOW.to(self.window_id));
        self.app_state.handle_event(event, self.window_id);
    }

    fn destroy(&mut self) {
        self.app_state.window_removed(self.window_id);
    }

    fn got_focus(&mut self) {
        self.app_state.window_got_focus(self.window_id);
    }

    fn prepare_paint(&mut self) {
        self.app_state.prepare_paint(self.window_id);
    }

    fn paint(&mut self, piet: &mut Piet, region: &Region) {
        self.app_state.paint(self.window_id, piet, region);
    }

    // TODO - rename to resize
    fn size(&mut self, size: Size) {
        let event = Event::WindowSize(size);
        self.app_state.handle_event(event, self.window_id);
    }

    // TODO - rename to rescale
    fn scale(&mut self, _scale: Scale) {
        // TODO: Do something with the scale
    }

    fn command(&mut self, id: u32) {
        self.app_state.handle_system_cmd(id, Some(self.window_id));
    }

    fn save_as(&mut self, token: FileDialogToken, file_info: Option<FileInfo>) {
        self.app_state.handle_dialog_response(token, file_info);
    }

    fn open_file(&mut self, token: FileDialogToken, file_info: Option<FileInfo>) {
        self.app_state.handle_dialog_response(token, file_info);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        // TODO: double-click detection (or is this done in druid-shell?)
        let event = Event::MouseDown(event.clone().into());
        self.app_state.handle_event(event, self.window_id);
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        let event = Event::MouseUp(event.clone().into());
        self.app_state.handle_event(event, self.window_id);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        let event = Event::MouseMove(event.clone().into());
        self.app_state.handle_event(event, self.window_id);
    }

    fn mouse_leave(&mut self) {
        self.app_state
            .handle_event(Event::Internal(InternalEvent::MouseLeave), self.window_id);
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        self.app_state
            .handle_event(Event::KeyDown(event), self.window_id)
            .is_handled()
    }

    fn key_up(&mut self, event: KeyEvent) {
        self.app_state
            .handle_event(Event::KeyUp(event), self.window_id);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        self.app_state
            .handle_event(Event::Wheel(event.clone().into()), self.window_id);
    }

    fn zoom(&mut self, delta: f64) {
        let event = Event::Zoom(delta);
        self.app_state.handle_event(event, self.window_id);
    }

    fn timer(&mut self, token: TimerToken) {
        self.app_state
            .handle_event(Event::Timer(token), self.window_id);
    }

    fn idle(&mut self, token: IdleToken) {
        match token {
            RUN_COMMANDS_TOKEN => {
                self.app_state.run_commands();
            }
            EXT_EVENT_IDLE_TOKEN => {
                self.app_state.run_ext_events();
            }
            other => {
                tracing::warn!("unexpected idle token {:?}", other);
            }
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn acquire_input_lock(
        &mut self,
        token: TextFieldToken,
        mutable: bool,
    ) -> Box<dyn InputHandler> {
        self.app_state.get_ime_lock(self.window_id, token, mutable)
    }

    fn release_input_lock(&mut self, token: TextFieldToken) {
        let needs_update = self.app_state.release_ime_lock(self.window_id, token);
        if let Some(widget) = needs_update {
            let event = Event::Internal(InternalEvent::RouteImeStateChange(widget));
            self.app_state.handle_event(event, self.window_id);
        }
    }
}
