// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The types which can be used as elements in a [`View`](crate::View)

/// A type which can be used as the `Element` associated type for a [`View`](crate::View).
///
/// It is expected that most libraries using `xilem_core` will have a generic
/// implementation of this trait for their widget type.
/// Additionally, this may also be implemented for additional types.
/// In Xilem (the user interface library)
///
/// This does require the reference type to be reborrowable.
///
/// In most cases, there will be a corresponding implementation of [`SuperElement<Self>`] for
/// some other type.
/// This will be the generic form of this type, which is used for the implementation of [`AnyView`].
///
/// [`AnyView`]: crate::AnyView
///
// TODO: Rename so that it doesn't conflict with the type parameter names
pub trait ViewElement {
    /// The reference form of this `Element` for editing.
    ///
    /// This is provided to [`View::rebuild`](crate::View::rebuild) and
    /// [`View::teardown`](crate::View::teardown).
    /// This enables greater flexibility in the use of the traits, such as
    /// for reference types which contain access to parent state.
    type Mut<'a>;
    /* /// Perform a reborrowing access to the reference type, which allows re-using the reference
    /// even if it gets passed to another function.
    ///
    /// This is the more general form of [`with_reborrow`](ViewElement::with_reborrow).
    /// See its documentation for more details.
    fn with_reborrow_val<R: 'static>(
        this: Self::Mut<'_>,
        f: impl FnOnce(Self::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R);

    /// Perform a reborrowing access to the reference type, which allows re-using the reference
    /// even if it gets passed to another function.
    ///
    /// The closure accepts a second parameter of type `&()` because of rustc issue [#49601].
    /// In this case, the diagnostic is probably incorrect.
    /// When calling this function, you can safely ignore this parameter (i.e. `|element, _| {...}`).
    ///
    /// If you need to get a return value from the .
    /// Unfortunately, it isn't possible to abstract over reborrowing without a closure or equivalent.
    ///
    /// [#49601](https://github.com/rust-lang/rust/issues/49601)
    fn with_reborrow(this: Self::Mut<'_>, f: impl FnOnce(Self::Mut<'_>)) -> Self::Mut<'_> {
        let (this, ()) = Self::with_reborrow_val(this, f);
        this
    } */
}

/// This element type is a superset of `Child`.
///
/// There are two primary use cases for this type:
/// 1) The dynamic form of the element type, used for [`AnyView`] and [`ViewSequence`]s.
/// 2) Additional, optional, information which can be added to an element type.
///    This will primarily be used in [`ViewSequence`] implementations.
///
/// [`AnyView`]: crate::AnyView
/// [`ViewSequence`]: crate::ViewSequence
pub trait SuperElement<Child>: ViewElement
where
    Child: ViewElement,
{
    /// Convert from the child to this element type.
    fn upcast(child: Child) -> Self;

    /// Perform a reborrowing downcast to the child reference type.
    ///
    /// This may panic if `this` is not the reference form of a value created by
    /// `Self::upcast`.
    /// For example, this may perform a downcasting operation, which would fail
    /// if the value is not of the expected type.
    /// You can safely use this methods in contexts where it is known that the
    ///
    /// If you need to return a value, see [`with_downcast_val`](SuperElement::with_downcast_val).
    fn with_downcast(this: Self::Mut<'_>, f: impl FnOnce(Child::Mut<'_>)) -> Self::Mut<'_> {
        let (this, ()) = Self::with_downcast_val(this, f);
        this
    }
    /// Perform a reborrowing downcast.
    ///
    /// This may panic if `this` is not the reference form of a value created by
    /// `Self::upcast`.
    ///
    /// If you don't need to return a value, see [`with_downcast`](SuperElement::with_downcast).
    fn with_downcast_val<R>(
        this: Self::Mut<'_>,
        f: impl FnOnce(Child::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R);
}

/// An element which can be used for an [`AnyView`]
pub trait AnyElement<Child>: SuperElement<Child>
where
    Child: ViewElement,
{
    /// Replace the inner value of this reference entirely
    fn replace_inner(this: Self::Mut<'_>, child: Child) -> Self::Mut<'_>;
}
// TODO: What do we want to do here? This impl seems nice, but is it necessary?
// It lets you trivially have sequences of types with a heterogenous element type,
// but how common are those in practice?
// It conflicts with the xilem_masonry dynamic implementation (assuming that `Box<dyn Widget>: Widget` holds)
// impl<E: Element> SuperElement<E> for E {
//     fn upcast(child: E) -> Self { child }
//     fn downcast<'a>(refm: Self::Mut<'a>) -> <E as Element>::Mut<'a> { refm }
// }
