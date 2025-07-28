// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, Environment, ViewId};
use alloc::{boxed::Box, vec::Vec};
use anymore::AnyDebug;

/// The `MessageContext` is used in [`View::message`](crate::View::message).
///
/// It contains the full current "target" path for message routing, along with
/// where we are along that path.
/// Additionally, it also provides access to the current [`Environment`],
/// allowing the resources for the current view tree location to be accessed.
// TODO: Is it OK for this debug to be load bearing? It probably shouldn't be a derive.
#[derive(Debug)]
pub struct MessageContext {
    // TODO: Just plain pub?
    pub(crate) environment: Environment,
    full_id_path: Vec<ViewId>,
    id_path_index: usize,
    message: Option<DynMessage>,
}

impl MessageContext {
    // TODO: Tests.
    /// Remove the first element from the id path which this message needs to be routed to.
    ///
    /// This mirrors [`ViewPathTracker::with_id`](crate::ViewPathTracker::with_id).
    /// Returns `None` if there are no more elements in the id path (which likely means that
    /// this view is the target view, depending on how it uses its section of the id path).
    pub fn take_first(&mut self) -> Option<ViewId> {
        let ret = self.full_id_path.get(self.id_path_index)?;
        self.id_path_index += 1;
        Some(*ret)
    }

    /// The remaining id path, which should mostly be handled by your children.
    ///
    /// If this returns an empty slice, then `take_first` will return `None`.
    pub fn remaining_path(&self) -> &[ViewId] {
        &self.full_id_path[self.id_path_index..]
    }

    /// The path to this view.
    pub fn current_path(&self) -> &[ViewId] {
        &self.full_id_path[..self.id_path_index]
    }
    /// Take the message, downcasting it to the specified type.
    ///
    /// If the message is not of the specified type, returns `None`.
    /// Should only be used, when the message has reached its target,
    /// i.e. `assert!(self.remaining_path().is_empty())`
    ///
    /// # Panics
    ///
    /// If the message has already been taken.
    #[track_caller]
    pub fn take_message<T: AnyDebug>(&mut self) -> Option<Box<T>> {
        self.maybe_take_message(|_| true)
    }

    /// Downcast the message to the specified type, taking it if `f` returns true.
    ///
    /// If the message is not of the specified type, returns `None`.
    ///
    /// # Panics
    ///
    /// If the message has already been taken.
    #[track_caller]
    pub fn maybe_take_message<T: AnyDebug>(
        &mut self,
        f: impl FnOnce(&T) -> bool,
    ) -> Option<Box<T>> {
        debug_assert_eq!(
            self.full_id_path.len(),
            self.id_path_index,
            "Can't take a message that has not reached it's target"
        );
        if let Some(message) = self.message.take() {
            if message.is::<T>() {
                let message = message.downcast().unwrap();
                if f(&*message) {
                    return Some(message);
                } else {
                    self.message = Some(DynMessage(message));
                }
            } else {
                self.message = Some(message);
            }
            None
        } else {
            panic!("The message has already been taken.");
        }
    }
}

/// Methods used by implementations of the Xilem pattern, not directly by View implementations.
impl MessageContext {
    /// Create a new message context.
    ///
    /// End-users of Xilem do not need to use this function.
    ///
    /// For driver implementers, the provided environment should your app's global environment.
    /// This can be recovered by [`finish`](Self::finish).
    pub fn new(environment: Environment, target_id_path: Vec<ViewId>, message: DynMessage) -> Self {
        Self {
            environment,
            full_id_path: target_id_path,
            id_path_index: 0,
            message: Some(message),
        }
    }

    /// Unwrap this `MessageContext` into its constituent parts.
    pub fn finish(self) -> (Environment, Vec<ViewId>, Option<DynMessage>) {
        let Self {
            environment,
            full_id_path,
            message,
            ..
        } = self;
        (environment, full_id_path, message)
    }
}
