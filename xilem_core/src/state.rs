// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// TODO: Different name for this module?
// TODO: All of these state related terms in Xilem (including likely even the `State` generic variable)
// need a renaming pass.

// TODO: Different name?
/// The arguments which a [`View`](crate::view::View) accepts.
///
/// This trait is used to talk about "reference" versions of states.
///
/// This is implemented for [`Edit<T>`], [`Read<T>`], and tuples thereof.
/// It can also be implemented manually, which allows names to be given to fields.
///
/// # Examples
///
/// In these examples, the Action and Context parameters to `View` are elided.
///
/// - `View<Edit<f32>>` will read and write one `f32` value.
/// - `View<(Edit<f32>, Range<f32>)>` will read and write one `f32` value, and read an `f32` range.
/// - `View<MyParameters<'static>>` will perform the operations described in `MyParameters`.
///
/// `MyParameters` in the example would look something like:
///
/// ```
/// use xilem_core::ViewArgument;
/// struct MyParameters<'a> {
///     parameter: &'a f32,
///     output: &'a mut f32,
///     other: &'a f32
/// }
///
/// impl ViewArgument for MyParameters<'static> {
///     type Params<'a> = MyParameters<'a>;
///
///     fn reborrow_mut<'input, 'a: 'input>(
///         params: &'input mut Self::Params<'a>,
///     ) -> Self::Params<'input> {
///         MyParameters {
///             parameter: &params.parameter,
///             output: &mut params.output,
///             other: &params.other,
///         }
///     }
/// }
/// ```
///
/// Note that in this example, `MyParameters<'static>` is used as a convenient type which can be implemented using a static lifetime.
/// Values of that static lifetimed version of the type is never constructed nor needed by Xilem.
/// When we explore this further, it may be more idiomatic to use a type alias such as `type MyParametersArg = MyParameters<'static>`
/// (or indeed, a `struct MyParametersArg;`)
pub trait ViewArgument: 'static {
    // TODO: Different name
    // TODO: This GAT existing seems to force T: 'static?
    // We could add a `'b` lifetime parameter to ViewArgument; not sure what
    // that semantically represents, though.
    /// The reference for of this argument, which is what is actually passed down the tree
    /// (with different lifetimes).
    type Params<'a>;

    /// Reborrow the parameters to a shorter lifetime, keeping the original around
    /// (for use once that lifetime ends).
    ///
    /// This is useful for passing the parameters to several child views, such as in a
    /// loop for multiple children.
    fn reborrow_mut<'input, 'a: 'input>(
        params: &'input mut Self::Params<'a>,
    ) -> Self::Params<'input>;
}

/// When used in the first "state" parameter to `View`, this type indicates that
/// the view value writes to a value of type `T` when performing reconciliation and/or event handling.
///
/// Note that this writing occurs using a standard exclusive reference (i.e. `&mut T`), which is checked
/// by the borrow-checker (i.e. this cannot dynamically fail - this is unlike some signal based solutions).
/// This type implements [`ViewArgument`], meaning that the view must be provided with that exclusive reference to operate.
///
/// # Examples
///
/// A simple component can edit a single value when an event happens:
///
/// ```rust
/// # use xilem_core::docs::{DocsView as WidgetView, stateless_component};
/// # use xilem_core::map_state;
/// use xilem_core::Edit;
/// fn button() -> impl WidgetView<Edit<f64>> {
///     // ...
/// #   map_state(stateless_component(), |_, ()| ())
/// }
/// ```
///
/// Most components are likely to accept multiple parameters.
/// Read-only parameters can use [`Read`].
///
/// ```rust
/// # use xilem_core::docs::{DocsView as WidgetView, stateless_component};
/// # use xilem_core::map_state;
/// use core::ops::Range;
/// use xilem_core::{Read, Edit};
/// fn slider() -> impl WidgetView<(Edit<f64>, Read<Range<f64>>)> {
///     // ...
/// #   map_state(stateless_component(), |(_result, _range), ()| ())
/// }
/// ```
// TODO: Maybe the name `Mut` here would make sense? I don't hate edit, though.
// TODO: This forces T to be 'static, even though that isn't actually needed.
#[doc(alias = "mutate")]
#[doc(alias = "write")]
pub type Edit<T> = &'static mut T;

// TODO: These docs could probably do with a clarifying pass.
/// When used in the first "state" parameter to `View`, this type indicates that
/// the view value reads a value of type `T` when performing reconciliation and/or event handling.
///
/// This type implements [`ViewArgument`], and so any view operations for views which
/// use this must be provided with a shared reference to `T` (i.e. `&T`).
/// See [`Edit`] for the equivalent exclusive version of this.
///
/// # Examples
///
/// A simple component can display a single result:
///
/// ```rust
/// # use xilem_core::docs::{DocsView as WidgetView, stateless_component};
/// # use xilem_core::map_state;
/// use xilem_core::Read;
/// fn display_result() -> impl WidgetView<Read<f64>> {
///     // ...
/// #   map_state(stateless_component(), |_, ()| ())
/// }
/// ```
///
/// A component accepting only read access to a single value might not be that useful.
/// Most components are likely to accept at least one [editable](Edit) parameter.
/// This can be achieved using a tuple:
///
/// ```rust
/// # use xilem_core::docs::{DocsView as WidgetView, stateless_component};
/// # use xilem_core::map_state;
/// use core::ops::Range;
/// use xilem_core::{Read, Edit};
/// fn slider() -> impl WidgetView<(Edit<f64>, Read<Range<f64>>)> {
///     // ...
/// #   map_state(stateless_component(), |(_result, _range), ()| ())
/// }
/// ```
pub type Read<T> = &'static T;

/// An alias to the "reference" form of a [`ViewArgument`].
///
/// This is used in `View` functions (instead of the expanded form) to decrease
/// the amount of noise produced.
// TODO: The name of this definitely needs rethinking.
pub type Arg<'a, T> = <T as ViewArgument>::Params<'a>;

// TODO: Do these need to be 'static?
impl<T> ViewArgument for &'static T {
    type Params<'a> = &'a T;
    fn reborrow_mut<'input, 'a: 'input>(
        params: &'input mut Self::Params<'a>,
    ) -> Self::Params<'input> {
        *params
    }
}

impl<T> ViewArgument for &'static mut T {
    type Params<'a> = &'a mut T;
    fn reborrow_mut<'input, 'a: 'input>(
        params: &'input mut Self::Params<'a>,
    ) -> Self::Params<'input> {
        &mut *params
    }
}

impl ViewArgument for () {
    type Params<'a> = ();
    fn reborrow_mut<'input, 'a: 'input>((): &'input mut Self::Params<'a>) -> Self::Params<'input> {}
}

impl<T0: ViewArgument> ViewArgument for (T0,) {
    type Params<'a> = (T0::Params<'a>,);
    fn reborrow_mut<'input, 'a: 'input>(
        (t0,): &'input mut Self::Params<'a>,
    ) -> Self::Params<'input> {
        (T0::reborrow_mut(t0),)
    }
}

impl<T0: ViewArgument, T1: ViewArgument> ViewArgument for (T0, T1) {
    type Params<'a> = (T0::Params<'a>, T1::Params<'a>);
    fn reborrow_mut<'input, 'a: 'input>(
        (t0, t1): &'input mut Self::Params<'a>,
    ) -> Self::Params<'input> {
        (T0::reborrow_mut(t0), T1::reborrow_mut(t1))
    }
}

// TODO: Generate 2+ with a macro; maybe 1+, but then again for understandability purposes, having at least one
// outside of the macro is appealing..
impl<T0: ViewArgument, T1: ViewArgument, T2: ViewArgument> ViewArgument for (T0, T1, T2) {
    type Params<'a> = (T0::Params<'a>, T1::Params<'a>, T2::Params<'a>);
    fn reborrow_mut<'input, 'a: 'input>(
        (t0, t1, t2): &'input mut Self::Params<'a>,
    ) -> Self::Params<'input> {
        (
            T0::reborrow_mut(t0),
            T1::reborrow_mut(t1),
            T2::reborrow_mut(t2),
        )
    }
}
