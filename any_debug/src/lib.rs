// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Provides the [`AnyDebug`] trait and blanket implementation.

use std::{any::Any, fmt::Debug};

/// A type that implements [`Any`], [`Debug`] and [`Send`].
///
/// The `Debug` requirement allows inspecting messages which were sent to the wrong place.
// TODO: Should/Could we remove the `Send` requirement here?
// It's not like implementing message handling in parallel is a meaningful operation.
// (If you need to send one, you can always use `dyn AnyMessage + Send`)
// Making that change would mean we could make View no longer generic over the message type again.
pub trait AnyDebug: Any + Debug + Send {}
impl<T> AnyDebug for T where T: Any + Debug + Send {}

impl dyn AnyDebug {
    /// Access the actual type.
    ///
    /// ## Errors
    ///
    /// If the message contained within `self` is not of type `T`, returns `self`
    /// (so that e.g. a different type can be used)
    pub fn downcast<T: AnyDebug>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.is::<T>() {
            Ok((self as Box<dyn Any>).downcast::<T>().unwrap())
        } else {
            Err(self)
        }
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: AnyDebug>(&self) -> bool {
        let this: &dyn Any = self;
        this.is::<T>()
    }
}

#[cfg(test)]
mod tests {
    use crate::AnyDebug;

    #[derive(Debug)]
    struct X;

    #[test]
    fn test_is() {
        let boxed = Box::new(X) as Box<dyn AnyDebug>;
        assert!(boxed.is::<X>());
    }

    #[test]
    fn test_downcast() {
        let boxed = Box::new(X) as Box<dyn AnyDebug>;
        boxed.downcast::<X>().unwrap();
    }
}
