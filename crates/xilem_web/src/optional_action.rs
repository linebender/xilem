/// Implement this trait for types you want to use as actions.
///
/// The trait exists because otherwise we couldn't provide versions
/// of listeners that take `()`, `A` and `Option<A>`.
pub trait Action {}

/// Trait that allows callbacks to be polymorphic on return type
/// (`Action`, `Option<Action>` or `()`). An implementation detail.
pub trait OptionalAction<A>: sealed::Sealed {
    fn action(self) -> Option<A>;
}
mod sealed {
    pub trait Sealed {}
}

impl sealed::Sealed for () {}
impl<A> OptionalAction<A> for () {
    fn action(self) -> Option<A> {
        None
    }
}

impl<A: Action> sealed::Sealed for A {}
impl<A: Action> OptionalAction<A> for A {
    fn action(self) -> Option<A> {
        Some(self)
    }
}

impl<A: Action> sealed::Sealed for Option<A> {}
impl<A: Action> OptionalAction<A> for Option<A> {
    fn action(self) -> Option<A> {
        self
    }
}
