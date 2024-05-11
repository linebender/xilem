// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

#[macro_export]
macro_rules! message {
    ($($bounds:tt)*) => {
        pub struct Message {
            pub id_path: xilem_core::IdPath,
            pub body: Box<dyn std::any::Any + $($bounds)*>,
        }

        impl Message {
            pub fn new(id_path: xilem_core::IdPath, event: impl std::any::Any + $($bounds)*) -> Message {
                Message {
                    id_path,
                    body: Box::new(event),
                }
            }
        }
    };
}

/// A result wrapper type for event handlers.
#[derive(Default)]
pub enum MessageResult<A> {
    /// The event handler was invoked and returned an action.
    ///
    /// Use this return type if your widgets should respond to events by passing
    /// a value up the tree, rather than changing their internal state.
    Action(A),
    /// The event handler received a change request that requests a rebuild.
    ///
    /// Note: A rebuild will always occur if there was a state change. This return
    /// type can be used to indicate that a full rebuild is necessary even if the
    /// state remained the same. It is expected that this type won't be used very
    /// often.
    #[allow(unused)]
    RequestRebuild,
    /// The event handler discarded the event.
    ///
    /// This is the variant that you **almost always want** when you're not returning
    /// an action.
    #[allow(unused)]
    #[default]
    Nop,
    /// The event was addressed to an id path no longer in the tree.
    ///
    /// This is a normal outcome for async operation when the tree is changing
    /// dynamically, but otherwise indicates a logic error.
    Stale(Box<dyn Any>),
}

// TODO: does this belong in core?
pub struct AsyncWake;

impl<A> MessageResult<A> {
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> MessageResult<B> {
        match self {
            MessageResult::Action(a) => MessageResult::Action(f(a)),
            MessageResult::RequestRebuild => MessageResult::RequestRebuild,
            MessageResult::Stale(event) => MessageResult::Stale(event),
            MessageResult::Nop => MessageResult::Nop,
        }
    }

    pub fn or(self, f: impl FnOnce(Box<dyn Any>) -> Self) -> Self {
        match self {
            MessageResult::Stale(event) => f(event),
            _ => self,
        }
    }
}
