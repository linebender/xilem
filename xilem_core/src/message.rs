// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Message routing and type erasure primitives.

use core::{any::Any, fmt::Debug, ops::Deref};

use alloc::boxed::Box;

/// The possible outcomes from a [`View::message`]
///
/// [`View::message`]: crate::View::message
#[derive(Default)]
pub enum MessageResult<Action> {
    /// An action for a parent message handler to use
    ///
    /// This allows for sub-sections of your app to use an elm-like architecture
    Action(Action),
    // TODO: What does this mean?
    /// This message's handler needs a rebuild to happen.
    /// The exact semantics of this method haven't been determined.
    RequestRebuild,
    #[default]
    /// This event had no impact on the app state, or the impact it did have
    /// does not require the element tree to be recreated.
    Nop,
    /// The view this message was being routed to no longer exists.
    Stale(DynMessage),
}

/// A dynamically typed message for the [`View`] trait.
///
/// Mostly equivalent to `Box<dyn Any>`, but with support for debug printing.
// We can't use intra-doc links here because of
/// The primary interface for this type is [`dyn Message::downcast`](trait.Message.html#method.downcast).
///
/// These messages must also be [`Send`].
/// This makes using this message type in a multithreaded context easier.
/// If this requirement is causing you issues, feel free to open an issue
/// to discuss.
/// We are aware of potential backwards-compatible workarounds, but
/// are not aware of any tangible need for this.
///
/// [`View`]: crate::View
pub type DynMessage = Box<dyn Message>;
/// Types which can be contained in a [`DynMessage`].
// The `View` trait could have been made generic over the message type,
// primarily to enable flexibility around Send/Sync and avoid the need
// for allocation.
pub trait Message: 'static {
    /// Convert `self` into a [`Box<dyn Any>`].
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    /// Convert `self` into a [`Box<dyn Any>`].
    fn as_any(&self) -> &(dyn Any);
    /// Gets the debug representation of this message.
    fn dyn_debug(&self) -> &dyn Debug;
}

impl<T> Message for T
where
    T: Any + Debug,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn dyn_debug(&self) -> &dyn Debug {
        self
    }
}

impl dyn Message {
    /// Access the actual type of this [`DynMessage`].
    ///
    /// In most cases, this will be unwrapped, as each [`View`](crate::View) will
    /// coordinate with their runner and/or element type to only receive messages
    /// of a single, expected, underlying type.
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be used)
    pub fn downcast<T: Message>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        // The panic is unreachable
        #![allow(clippy::missing_panics_doc)]
        if self.deref().as_any().is::<T>() {
            Ok(self
                .into_any()
                .downcast::<T>()
                .expect("`as_any` should correspond with `into_any`"))
        } else {
            Err(self)
        }
    }
}

impl Debug for dyn Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inner = self.dyn_debug();
        f.debug_tuple("Message").field(&inner).finish()
    }
}

/* /// Types which can route a message to a child [`View`].
// TODO: This trait needs to exist for desktop hot reloading
// This would be a supertrait of View
pub trait ViewMessage<State, Action> {
    type ViewState;
}
*/

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use alloc::boxed::Box;

    use crate::DynMessage;

    struct MyMessage(String);

    impl Debug for MyMessage {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("A present message")
        }
    }

    #[derive(Debug)]
    struct NotMyMessage;

    #[test]
    /// Downcasting a message to the correct type should work
    fn message_downcast() {
        let message: DynMessage = Box::new(MyMessage("test".to_string()));
        let result: Box<MyMessage> = message.downcast().unwrap();
        assert_eq!(&result.0, "test");
    }
    #[test]
    /// Downcasting a message to the wrong type shouldn't panic
    fn message_downcast_wrong_type() {
        let message: DynMessage = Box::new(MyMessage("test".to_string()));
        let message = message.downcast::<NotMyMessage>().unwrap_err();
        let result: Box<MyMessage> = message.downcast().unwrap();
        assert_eq!(&result.0, "test");
    }

    #[test]
    /// DynMessage's debug should pass through the debug implementation of
    fn message_debug() {
        let message: DynMessage = Box::new(MyMessage("".to_string()));
        let debug_result = format!("{message:?}");
        // Note that we
        assert!(debug_result.contains("A present message"));
    }
}
