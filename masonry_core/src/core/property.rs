// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::default::Default;

/// A marker trait that indicates that a type is intended to be used as a widget's property.
///
/// Properties are arbitrary values that are stored alongside a widget.
///
/// Note that if a type `Foobar` implements `Property`, that tells you that `Foobar` is meant
/// to be a property of *some* widget, but it doesn't tell you *which* widget accepts `Foobar`
/// as a property.
/// That information is deliberately not encoded in the type system.
/// We might change that in a future version.
pub trait Property: Default + Clone + Send + Sync + 'static {
    /// A static reference to a default value.
    ///
    /// Should be the same as [`Default::default()`].
    ///
    /// The reason we have this method is that we want e.g. [`PropertiesRef::get()`] to
    /// return a `&T`, not an `Option<&T>`.
    ///
    /// Therefore `Property` must provide a method that will return a `&'static T` that we
    /// can use anywhere.
    /// We do that is by creating a static inside each impl, and returning a reference to that static.
    ///
    /// Ideally, when const generics are stable, we'll want to use `const Default` directly in the default impl.
    fn static_default() -> &'static Self;

    /// Returns `true` if the given `property_type` matches this property.
    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

/// A marker trait indicating that the widget this is implemented for supports the property `P`.
///
/// You should implement this for your widget types, with each property the widget reads.
/// This is not used directly by Masonry Core, but is instead provided for the convenience of external crates.
pub trait HasProperty<P: Property> {}
