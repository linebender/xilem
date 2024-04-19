// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Simple handle for submitting external events.
#![allow(unused)]

use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use druid_shell::IdleHandle;

use crate::promise::PromiseResult;
use crate::widget::WidgetId;

// FIXME - Remove
pub struct WindowId;

pub(crate) enum ExtMessage {
    Promise(PromiseResult, WidgetId, WindowId),
}

/// A thing that can move into other threads and be used to submit commands back
/// to the running application.
///
/// This API is preliminary, and may be changed or removed without warning.
#[derive(Clone)]
pub struct ExtEventSink {
    queue: Arc<Mutex<VecDeque<ExtMessage>>>,
    handle: Arc<Mutex<Option<IdleHandle>>>,
}

/// The stuff that we hold onto inside the app that is related to the
/// handling of external events.
#[derive(Default)]
pub(crate) struct ExtEventQueue {
    /// A shared queue of items that have been sent to us.
    queue: Arc<Mutex<VecDeque<ExtMessage>>>,
    /// This doesn't exist when the app starts and it can go away if a window closes, so we keep a
    /// reference here and can update it when needed. Note that this reference is shared with all
    /// `ExtEventSink`s, so that we can update them too.
    handle: Arc<Mutex<Option<IdleHandle>>>,
    /// The window that the handle belongs to, so we can keep track of when
    /// we need to get a new handle.
    pub(crate) handle_window_id: Option<WindowId>,
}

/// An error that occurs if an external event cannot be submitted.
/// This probably means that the application has gone away.
#[derive(Debug, Clone)]
pub struct ExtEventError;

impl ExtEventQueue {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn make_sink(&self) -> ExtEventSink {
        ExtEventSink {
            queue: self.queue.clone(),
            handle: self.handle.clone(),
        }
    }

    pub(crate) fn set_idle(&mut self, handle: IdleHandle, window_id: WindowId) {
        self.handle.lock().unwrap().replace(handle);
        self.handle_window_id = Some(window_id);
    }

    pub(crate) fn has_pending_items(&self) -> bool {
        !self.queue.lock().unwrap().is_empty()
    }

    pub(crate) fn recv(&mut self) -> Option<ExtMessage> {
        self.queue.lock().unwrap().pop_front()
    }
}

impl ExtEventSink {
    #[allow(missing_docs)]
    pub fn resolve_promise(
        &self,
        result: PromiseResult,
        target_widget: WidgetId,
        target_window: WindowId,
    ) -> Result<(), ExtEventError> {
        if let Some(handle) = self.handle.lock().unwrap().as_mut() {
            todo!()
        }
        self.queue
            .lock()
            .map_err(|_| ExtEventError)?
            .push_back(ExtMessage::Promise(result, target_widget, target_window));
        Ok(())
    }
}

impl std::fmt::Display for ExtEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Window missing for external event")
    }
}

impl std::error::Error for ExtEventError {}
