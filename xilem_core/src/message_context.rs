// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::boxed::Box;
use alloc::vec::Vec;

use anymore::AnyDebug;

use crate::{DynMessage, Environment, ViewId};

/// The `MessageCtx` is used in [`View::message`](crate::View::message).
///
/// It contains the full current "target" path for message routing, along with
/// where we are along that path.
/// Additionally, it also provides access to the current [`Environment`],
/// allowing the resources for the current view tree location to be accessed.
// TODO: Is it OK for this debug to be load bearing? It probably shouldn't be a derive.
#[derive(Debug)]
pub struct MessageCtx {
    // TODO: Just plain pub?
    pub(crate) environment: Environment,
    full_id_path: Vec<ViewId>,
    id_path_index: usize,
    message: Option<DynMessage>,
}

impl MessageCtx {
    /// Removes the first element from the id path which this message needs to be routed to.
    ///
    /// This mirrors [`ViewPathTracker::with_id`](crate::ViewPathTracker::with_id).
    /// Returns `None` if there are no more elements in the id path (for views
    /// which follow the usual patterns in that case the calling view would be
    /// the target view).
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

    /// The id path to this view from the root.
    pub fn current_path(&self) -> &[ViewId] {
        &self.full_id_path[..self.id_path_index]
    }

    /// Takes the message, downcasting it to the specified type.
    ///
    /// If the message is not of the specified type, returns `None`.
    ///
    /// # Panics
    ///
    /// - If the message has already been taken.
    /// - If the message is not fully routed (i.e. the remaining path is not empty)
    #[track_caller]
    pub fn take_message<T: AnyDebug>(&mut self) -> Option<Box<T>> {
        self.maybe_take_message(|_| true)
    }

    /// Downcasts the message to the specified type, taking it if `f` returns true.
    ///
    /// If the message is not of the specified type, returns `None`.
    ///
    /// # Panics
    ///
    /// - If the message has already been taken.
    /// - If the message is not fully routed (i.e. the remaining path is not empty)
    #[track_caller]
    pub fn maybe_take_message<T: AnyDebug>(
        &mut self,
        f: impl FnOnce(&T) -> bool,
    ) -> Option<Box<T>> {
        debug_assert_eq!(
            self.full_id_path.len(),
            self.id_path_index,
            "Can't take a message that has not reached its target"
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

    /// Takes the message, or returns `None` if it's already been taken.
    ///
    /// This method is an escape hatch for [`take_message`](Self::take_message)
    /// and [`maybe_take_message`](Self::maybe_take_message).
    /// Almost all views should use those methods instead.
    #[track_caller]
    pub fn force_take_message<T: AnyDebug>(&mut self) -> Option<DynMessage> {
        self.message.take()
    }
}

/// Methods used by implementations of the Xilem pattern, not directly by View implementations.
impl MessageCtx {
    /// Creates a new message context.
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

    /// Unwraps this `MessageCtx` into its constituent parts.
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

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::{DynMessage, Environment, MessageCtx, ViewId};

    #[test]
    fn take_path_full_path() {
        let env = Environment::new();
        let path = [0, 4, 3, 2, 1, 0]
            .into_iter()
            .map(ViewId::new)
            .collect::<Vec<_>>();

        let mut ctx = MessageCtx::new(env, path.clone(), DynMessage::new(()));
        for element in &path {
            let next = ctx.take_first().unwrap();
            assert_eq!(next, *element);

            assert!(path.starts_with(ctx.current_path()));
            assert!(path.ends_with(ctx.remaining_path()));
            assert_eq!(
                path.len(),
                ctx.current_path().len() + ctx.remaining_path().len()
            );
            assert_eq!(*ctx.current_path().last().unwrap(), next);
        }
        assert!(ctx.take_first().is_none());
    }

    #[test]
    #[cfg_attr(
        not(debug_assertions),
        ignore = "This test doesn't work without debug assertions (i.e. in release mode)"
    )]
    #[should_panic(expected = "Can't take a message that has not reached its target")]
    fn take_message_nonempty_path() {
        let env = Environment::new();
        let path = vec![ViewId::new(1)];

        let mut ctx = MessageCtx::new(env, path.clone(), DynMessage::new(()));
        ctx.take_message::<()>();
    }

    #[test]
    fn take_message_wrong_type() {
        let env = Environment::new();
        let path = vec![];

        let mut ctx = MessageCtx::new(env, path.clone(), DynMessage::new(()));
        let took = ctx.take_message::<u32>();
        assert!(took.is_none());
        let () = *ctx.take_message::<()>().unwrap();
    }

    #[test]
    #[should_panic(expected = "The message has already been taken.")]
    fn take_message_twice() {
        let env = Environment::new();
        let path = vec![];

        let mut ctx = MessageCtx::new(env, path.clone(), DynMessage::new(()));
        let () = *ctx.take_message::<()>().unwrap();
        ctx.take_message::<()>();
    }

    #[test]
    fn maybe_take_message() {
        let env = Environment::new();
        let path = vec![];

        let mut ctx = MessageCtx::new(env, path.clone(), DynMessage::new(10_u32));
        ctx.maybe_take_message::<u32>(|x| {
            assert_eq!(*x, 10);
            false
        });
        let ret = ctx.take_message::<u32>().unwrap();
        assert_eq!(*ret, 10);
    }
}
