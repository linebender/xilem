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
/// This is the generic form of this type, which is used for the implementation of [`AnyView`].
pub trait Element {
    type Mut<'a>;
    /// Perform a reborrowing access to the reference type, which allows re-using the reference
    /// even if it gets passed to another function.
    ///
    /// This is the more general form of [`with_reborrow`](Element::with_reborrow).
    /// See its documentation for more details.
    fn with_reborrow_val<'o, R: 'static>(
        this: Self::Mut<'o>,
        f: impl FnOnce(Self::Mut<'_>) -> R,
    ) -> (Self::Mut<'o>, R);

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
    fn with_reborrow<'o>(this: Self::Mut<'o>, f: impl FnOnce(Self::Mut<'_>)) -> Self::Mut<'o> {
        let (this, ()) = Self::with_reborrow_val(this, f);
        this
    }
}

/// This element type is a superset of `Child`.
///
/// There are two primary use cases for this type:
/// 1) The dynamic form of the element type, used for [`AnyView`] and [`ViewSequence`]s.
/// 2) Additional, optional, information which can be added to an element type.
///    This will primarily be used in [`ViewSequence`] implementations.
///
/// [`AnyView`]: crate::AnyView
pub trait SuperElement<Child>: Element
where
    Child: Element,
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
    fn with_downcast<'a>(this: Self::Mut<'a>, f: impl FnOnce(Child::Mut<'_>)) -> Self::Mut<'a> {
        let (this, ()) = Self::with_downcast_val(this, f);
        this
    }
    /// Perform a reborrowing downcast.
    ///
    /// This may panic if `this` is not the reference form of a value created by
    /// `Self::upcast`.
    ///
    /// If you don't need to return a value, see [`with_downcast`](SuperElement::with_downcast).
    fn with_downcast_val<'a, R>(
        this: Self::Mut<'a>,
        f: impl FnOnce(Child::Mut<'_>) -> R,
    ) -> (Self::Mut<'a>, R);
}
