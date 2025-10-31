// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// TODO: Different name for this module?

use core::marker::PhantomData;

// TODO: Different name?
/// The arguments which a [`View`](crate::view::View) accepts.
// TODO: More docs
pub trait ViewArgument: 'static {
    // TODO: Different name
    // TODO: This GAT existing seems to force T: 'static?
    // We could add a `'b` lifetime parameter to ViewArgument; not sure what
    // that semantically represents, though.
    type Params<'a>;

    fn reborrow_mut<'input, 'a: 'input>(
        params: &'input mut Self::Params<'a>,
    ) -> Self::Params<'input>;
}

// TODO: Maybe the name `Mut` here would make sense? I don't hate edit, though.
// TODO: This forces T to be 'static, even though that isn't actually needed.
pub type Edit<T> = &'static mut T;

pub struct TempEdit<T>(PhantomData<fn() -> T>);

// TODO: Ideally, we'd use `Mut` here, but that clashes with `crate::Mut`
pub type Read<T> = &'static T;

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
