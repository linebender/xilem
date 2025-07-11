// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Message routing and type erasure primitives.

use alloc::boxed::Box;
use core::any::Any;
use core::fmt::Debug;

/// The possible outcomes from a [`View::message`]
///
/// [`View::message`]: crate::View::message
#[derive(Default, Debug)]
pub enum MessageResult<Action> {
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
    Stale(DynMessage),
}

impl<A> MessageResult<A> {
    /// Maps the action type `A` to `B`, i.e. [`MessageResult<A>`] to [`MessageResult<B>`]
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> MessageResult<B> {
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
///
/// To convert a `DynMessage` into its concrete message type, you should use
/// [`downcast`](Self::downcast).
///
/// If the message contains sensitive data, make sure this isn't output in its `Debug` implementation,
/// as that may be called by the Xilem runtime (e.g. due to a bug meaning messages are redirected) or
/// any parent views. That is, views do not need to be designed as if the `Debug` implementation
/// should only be logged securely, or shouldn't be sent to an anomaly reporting service.
///
/// [`View`]: crate::View
#[derive(Debug)]
// This type is a struct rather than (say) a type alias, because type aliases are sometimes resolved by
// rust-analyzer when autofilling a trait, and we want to always use a consistent name for this type.
pub struct DynMessage(pub Box<dyn AnyMessage>);

impl DynMessage {
    /// Utility to make a `DynMessage` from a message value.
    pub fn new(x: impl AnyMessage) -> Self {
        Self(Box::new(x))
    }

    /// Access the actual type of this [`DynMessage`].
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be checked).
    ///
    /// In most cases, to handle this error, you will want to make an `error` log,
    /// and return this as [`MessageResult::Stale`]; this case indicates that a parent
    /// view has routed things incorrectly, but it's reasonable to be robust.
    pub fn downcast<T: AnyMessage>(self) -> Result<Box<T>, Self> {
        self.0.downcast().map_err(Self)
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyMessage>(&self) -> bool {
        self.0.is::<T>()
    }
}

// We could consider:
// ```
// enum DynMessage {
//     Special(Box<dyn AnyMessage>),
//     Send(SendMessage)
// }
// ```
// to let "maybe-threaded" message handling. That would be especially useful for
// handling stale messages (i.e. reporting them back to the task which failed).
// Probably not worth it, but would be (reasonably) non-breaking, at least.
// Alternatively, we could pass a `fn(DynMessage)->Result<SendMessage, DynMessage>` to the
// main thread, which would assume/validate that the type hasn't changed. That is more
// fragile, but potentially more correct.

/// A dynamically typed message which can be sent between threads, for use in
/// reporting the results of asynchronous computation.
///
/// As in [`DynMessage`], this is a thin wrapper around `Box<dyn Any>`, with added
/// support for debug printing. It can be cheaply converted into a `DynMessage` using
/// the `From` implementation, although the opposite operation is not possible
/// (without knowing the underlying type). See also the warning in `DynMessage`'s
/// docs about the security of Debug implementations.
///
/// To convert a `SendMessage` into its concrete message type, you should use
/// [`downcast`](Self::downcast).
#[derive(Debug)]
pub struct SendMessage(pub Box<dyn AnyMessage + Send>);

impl From<SendMessage> for DynMessage {
    fn from(value: SendMessage) -> Self {
        Self(value.0)
    }
}

impl SendMessage {
    /// Utility to make a `SendMessage` from a message value.
    pub fn new(x: impl AnyMessage + Send) -> Self {
        Self(Box::new(x))
    }

    /// Access the actual type of this [`SendMessage`].
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be checked).
    pub fn downcast<T: AnyMessage>(self) -> Result<Box<T>, Self> {
        self.0.downcast().map_err(Self)
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyMessage + Send>(&self) -> bool {
        self.0.is::<T>()
    }
}

/// Types which can be used in [`DynMessage`] (and so can be the messages for Xilem views).
///
/// The `Debug` requirement allows inspecting messages which were sent to the wrong place.
// TODO: Rename to `AnyDebug`.
pub trait AnyMessage: Any + Debug {}
impl<T> AnyMessage for T where T: Any + Debug {}

impl dyn AnyMessage {
    /// Returns some reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_ref<T: AnyMessage>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    /// Returns some reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_mut<T: AnyMessage>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }

    /// Access the actual type of this [`AnyMessage`].
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be used)
    pub fn downcast<T: AnyMessage>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.is::<T>() {
            Ok((self as Box<dyn Any>).downcast::<T>().unwrap())
        } else {
            Err(self)
        }
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyMessage>(&self) -> bool {
        let this: &dyn Any = self;
        this.is::<T>()
    }
}

impl dyn AnyMessage + Send {
    /// Returns some reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_ref<T: AnyMessage>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    /// Returns some reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_mut<T: AnyMessage>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }

    /// Access the actual type of this [`AnyMessage`].
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be used)
    // We don't require Send here to mirror the standard library
    pub fn downcast<T: AnyMessage>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.is::<T>() {
            Ok((self as Box<dyn Any>).downcast::<T>().unwrap())
        } else {
            Err(self)
        }
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyMessage>(&self) -> bool {
        let this: &dyn Any = self;
        this.is::<T>()
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
