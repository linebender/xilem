// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, fmt::Debug, ops::Deref};

/// A dynamically typed message for the [`View`] trait.
///
/// Mostly equivalent to `Box<dyn Any>`, but with support for debug printing.
// We can't use intra-doc links here because of
/// The primary interface for this type is [`dyn Message::downcast`](trait.Message.html#method.downcast).
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
