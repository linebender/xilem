// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(unused)]
use std::collections::HashMap;

use tracing::trace;

use crate::action::Action;
use crate::command::{Command, CommandQueue};
use crate::ext_event::{ExtEventQueue, ExtEventSink};
use crate::widget::{StoreInWidgetMut, WidgetMut, WidgetRef};
use crate::{
    Event, Handled, SingleUse, Target, Widget, WidgetId, WindowDescription, WindowId, WindowRoot,
};

/// A context provided to [`AppDelegate`] methods.
pub struct DelegateCtx<'a, 'b> {
    //pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) ext_event_queue: &'a ExtEventQueue,
    // FIXME - Ideally, we'd like to get a hashmap of all root widgets,
    // but that creates "aliasing mutable references" problems
    // See issue #17
    pub(crate) main_root_widget: WidgetMut<'a, 'b, Box<dyn Widget>>,
    //pub(crate) active_windows: &'a mut HashMap<WindowId, WindowRoot>,
}

impl<'a, 'b> DelegateCtx<'a, 'b> {
    #[cfg(FALSE)]
    pub fn submit_command(&mut self, command: impl Into<Command>) {
        self.command_queue
            .push_back(command.into().default_to(Target::Global))
    }

    /// Return an [`ExtEventSink`] that can be moved between threads,
    /// and can be used to submit commands back to the application.
    pub fn get_external_handle(&self) -> ExtEventSink {
        self.ext_event_queue.make_sink()
    }

    #[cfg(FALSE)]
    pub fn new_window(&mut self, desc: WindowDescription) {
        trace!("new_window");
        self.submit_command(
            crate::command::NEW_WINDOW
                .with(SingleUse::new(Box::new(desc)))
                .to(Target::Global),
        );
    }

    // TODO - Use static typing to guarantee proper return type - See issue #17
    /// Try to return a [`WidgetMut`] to the root widget.
    ///
    /// Returns null if the returned type doesn't match the root widget type.
    pub fn try_get_root<W: Widget + StoreInWidgetMut>(&mut self) -> Option<WidgetMut<'_, 'b, W>> {
        self.main_root_widget.downcast()
    }

    /// Return a [`WidgetMut`] to the root widget.
    ///
    /// ## Panics
    ///
    /// Panics if the returned type doesn't match the root widget type.
    pub fn get_root<W: Widget + StoreInWidgetMut>(&mut self) -> WidgetMut<'_, 'b, W> {
        self.main_root_widget.downcast().expect("wrong widget type")
    }
}

/// A type that provides hooks for handling top-level events.
///
/// The `AppDelegate` is a trait that is allowed to handle and filter events before
/// they are passed down the widget tree.
pub trait AppDelegate {
    /// The handler for non-command [`Event`]s.
    ///
    /// This function receives all non-command events, before they are passed down
    /// the tree. If it returns [`Handled::Yes`], events are short-circuited.
    fn on_event(&mut self, ctx: &mut DelegateCtx, window_id: WindowId, event: &Event) -> Handled {
        #![allow(unused)]
        Handled::No
    }

    /// The handler for [`Command`]s.
    ///
    /// This function receives all command events, before they are passed down
    /// the tree. If it returns [`Handled::Yes`], commands are short-circuited.
    fn on_command(&mut self, ctx: &mut DelegateCtx, cmd: &Command) -> Handled {
        #![allow(unused)]
        Handled::No
    }

    /// The handler for [`Action`]s.
    ///
    /// Note: Actions are still a WIP part of masonry.
    fn on_action(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        widget_id: WidgetId,
        action: Action,
    ) {
        #![allow(unused)]
    }

    /// The handler for window creation events.
    ///
    /// This function is called after a window has been added,
    /// allowing you to customize the window creation behavior of your app.
    fn on_window_added(&mut self, ctx: &mut DelegateCtx, id: WindowId) {
        #![allow(unused)]
    }

    /// The handler for window deletion events.
    ///
    /// This function is called after a window has been removed.
    fn on_window_removed(&mut self, ctx: &mut DelegateCtx, id: WindowId) {
        #![allow(unused)]
    }
}

// TODO - impl AppDelegate for FnMut

// TODO - document
pub(crate) struct NullDelegate;

impl AppDelegate for NullDelegate {}
