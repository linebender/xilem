// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The types which can be used as elements in a [`View`](crate::View)

/// A type which can be used as the `Element` associated type for a [`View`](crate::View).
///
/// It is expected that most libraries using `xilem_core` will have a generic
/// implementation of this trait for their widget type.
/// Additionally, this may also be implemented for other types, depending on the
/// needs of the specific parent view.
/// In Xilem (the user interface library), this is also used for types containing the
/// flex properties of their child views, and window properties.
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
}

/// This alias is syntax sugar to avoid the elaborate expansion of
/// `<Self::Element as ViewElement>::Mut<'el>` in the View trait when implementing it (e.g. via rust-analyzer)
pub type Mut<'el, E> = <E as ViewElement>::Mut<'el>;

/// This element type is a superset of `Child`.
///
/// There are two primary use cases for this type:
/// 1) The dynamic form of the element type, used for [`AnyView`] and [`ViewSequence`]s.
/// 2) Additional, optional, information which can be added to an element type.
///    This will primarily be used in [`ViewSequence`] implementations.
///
/// [`AnyView`]: crate::AnyView
/// [`ViewSequence`]: crate::ViewSequence
pub trait SuperElement<Child, Context>: ViewElement
where
    Child: ViewElement,
{
    /// Convert from the child to this element type.
    fn upcast(ctx: &mut Context, child: Child) -> Self;

    /// Perform a reborrowing downcast to the child reference type.
    ///
    /// This may panic if `this` is not the reference form of a value created by
    /// `Self::upcast`.
    /// For example, this may perform a downcasting operation, which would fail
    /// if the value is not of the expected type.
    /// You can safely use this methods in contexts where it is known that the
    ///
    /// If you need to return a value, see [`with_downcast_val`](SuperElement::with_downcast_val).
    fn with_downcast(this: Mut<'_, Self>, f: impl FnOnce(Mut<'_, Child>)) -> Mut<'_, Self> {
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
        this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Child>) -> R,
    ) -> (Self::Mut<'_>, R);
}

/// An element which can be used for an [`AnyView`](crate::AnyView) containing `Child`.
pub trait AnyElement<Child, Context>: SuperElement<Child, Context>
where
    Child: ViewElement,
{
    /// Replace the inner value of this reference entirely
    fn replace_inner(this: Self::Mut<'_>, child: Child) -> Self::Mut<'_>;
}

/// Element type for views which don't impact the element tree.
///
/// Views with this element type can be included in any [`ViewSequence`](crate::ViewSequence) (with the
/// correct `State` and `Action` types), as they do not need to actually add an element to the sequence.
///
/// These views can also as the `alongside_view` in [`fork`](crate::fork).
pub struct NoElement;

impl ViewElement for NoElement {
    type Mut<'a> = ();
}

impl<Context> SuperElement<NoElement, Context> for NoElement {
    fn upcast(_ctx: &mut Context, child: NoElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, NoElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        ((), f(this))
    }
}
