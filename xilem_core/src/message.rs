// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Message routing and type erasure primitives.

use alloc::boxed::Box;
use any_debug::AnyDebug;
use core::fmt::Debug;

/// The possible outcomes from a [`View::message`]
///
/// [`View::message`]: crate::View::message
#[derive(Default, Debug)]
pub enum MessageResult<Action, Message = DynMessage> {
    /// An action for a parent message handler to use
    ///
    /// This allows for sub-sections of your app to use an elm-like architecture
    Action(Action),
    /// A view has requested a rebuild, even though its value hasn't changed.
    ///
    /// This can happen for example by some kind of async action.
    /// An example would be an async virtualized list, which fetches new entries, and requires a rebuild for the new entries.
    RequestRebuild,
    #[default]
    /// This event had no impact on the app state, or the impact it did have
    /// does not require the element tree to be recreated.
    Nop,
    /// The view this message was being routed to no longer exists.
    Stale(Message),
}

impl<A, Message> MessageResult<A, Message> {
    /// Maps the action type `A` to `B`, i.e. [`MessageResult<A>`] to [`MessageResult<B>`]
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> MessageResult<B, Message> {
        match self {
            Self::Action(a) => MessageResult::Action(f(a)),
            Self::RequestRebuild => MessageResult::RequestRebuild,
            Self::Stale(message) => MessageResult::Stale(message),
            Self::Nop => MessageResult::Nop,
        }
    }
}

/// A simple dynamically typed message for the [`View`] trait.
///
/// This is a thin wrapper around `Box<dyn Any>`, with added support for debug printing.
/// It is used as the default message type in Xilem Core.
/// The contained messages must also be [`Send`], which makes using this message type in a multithreaded context easier.
/// [`View`] is generic over the message type, in case this requirement is too restrictive.
/// Indeed, this functionality is used in Xilem Web.
///
/// To convert a `DynMessage` into its concrete message type, you should use
/// [`downcast`](Self::downcast).
///
/// This type is a struct rather than (say) a type alias, because type aliases are sometimes resolved by
/// rust-analyzer when autofilling a trait, which can also lead to buggy behaviour (we've previously seen
/// `Box<dyn Box<dyn Message>>` be generated).
///
/// If the message contains sensitive data, make sure this isn't output in its `Debug` implementation,
/// as that may be called by the Xilem runtime (e.g. due to a bug meaning messages are redirected) or
/// any parent views. (That is, views do not need to design themselves as if the Debug implementation is )
///
/// [`View`]: crate::View
#[derive(Debug)]
pub struct DynMessage(pub Box<dyn AnyDebug>);

impl DynMessage {
    /// Utility to make a `DynMessage` from a message value.
    pub fn new(x: impl AnyDebug) -> Self {
        Self(Box::new(x))
    }

    /// Access the actual type of this [`DynMessage`].
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be used).
    ///
    /// In most cases, to handle this error, you will want to make an `error` log,
    /// and return this as [`MessageResult::Stale`]; this case indicates that a parent
    /// view has routed things incorrectly, but it's reasonable to be robust.
    pub fn downcast<T: AnyDebug>(self) -> Result<Box<T>, Self> {
        self.0.downcast().map_err(Self)
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyDebug>(&self) -> bool {
        self.0.is::<T>()
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::{String, ToString};
    use core::fmt::Debug;

    use crate::DynMessage;

    struct MyMessage(String);

    impl Debug for MyMessage {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!("A present message: {:?}", self.0))
        }
    }

    #[derive(Debug)]
    struct NotMyMessage;

    #[test]
    /// Downcasting a message to the correct type should work.
    fn message_downcast() {
        let message = DynMessage::new(MyMessage("test".to_string()));
        let result: Box<MyMessage> = message.downcast().unwrap();
        assert_eq!(&result.0, "test");
    }
    #[test]
    /// Downcasting a message to the wrong type shouldn't panic, and should allow
    /// using the message with the right type.
    fn message_downcast_wrong_type() {
        let message = DynMessage::new(MyMessage("test".to_string()));
        let message = message.downcast::<NotMyMessage>().unwrap_err();
        let result: Box<MyMessage> = message.downcast().unwrap();
        assert_eq!(&result.0, "test");
    }

    #[test]
    /// `DynMessage`'s debug should pass through the debug implementation of.
    fn message_debug() {
        let message = DynMessage::new(MyMessage("".to_string()));
        let debug_result = format!("{message:?}");

        assert!(debug_result.contains("A present message"));
    }
}
