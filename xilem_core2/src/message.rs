// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::{any::Any, fmt::Debug};

use alloc::boxed::Box;

/// A dynamically typed message for the [`View`] trait.
///
/// This is the default message type for that trait, and should be
/// used in cases where that is required.
///
/// Mostly equivalent to [`Any`], but with support for debug printing.
///
/// These messages must also be [`Send`].
/// This makes using this message type in a multithreaded context easier.
/// If this requirement is problematic, you can use an alternative type
/// as the `Message` parameter to [`View`].
///
/// [`View`]: crate::View
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

pub type DynMessage = Box<dyn Message>;

impl dyn Message {
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
