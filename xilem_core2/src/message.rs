// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::{any::Any, fmt::Debug};

use alloc::boxed::Box;

/// The possible outcomes from a [`View::message`]
#[derive(Default)]
pub enum MessageResult<Action> {
    /// An action for a parent message handler to use
    ///
    /// This allows for sub-sections of your app to use an elm-like architecture
    Action(Action),
    /// This event had no impact on the app state, or the impact it did have
    /// does not require the element tree to be recreated.
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
pub trait Message: 'static + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any + Send>;
    fn as_any(&self) -> &dyn Any;
    fn dyn_debug(&self) -> &dyn Debug;
}

impl<T> Message for T
where
    T: Any + Debug + Send,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any + Send> {
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
    /// In most cases, this can be safely unwrapped, as each [`View`](crate::View) will
    /// only receive messages of a single type
    pub fn downcast<T: Message>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.as_any().is::<T>() {
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
// This would be a supertrait
pub trait ViewMessage<State, Action> {
    type ViewState;
}
*/
