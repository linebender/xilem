// TODO document everything, possibly different naming
#![allow(missing_docs)]
// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId, ViewPathTracker};

// TODO it would be nice to be able to decide between those two ways (mostly to avoid allocations, as otherwise `as_view` would be better I think)

/// A way to implement `View` for foreign types
pub trait AsOrphanView<T, State, Action>: ViewPathTracker + Sized {
    type V: View<State, Action, Self>;
    fn as_view(value: &T) -> Self::V;
}

pub trait OrphanView<T, State, Action>: ViewPathTracker + Sized {
    type Element: ViewElement;
    type ViewState;

    fn build(view: &T, ctx: &mut Self) -> (Self::Element, Self::ViewState);
    fn rebuild<'el>(
        new: &T,
        prev: &T,
        view_state: &mut Self::ViewState,
        ctx: &mut Self,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element>;

    fn teardown(
        view: &T,
        view_state: &mut Self::ViewState,
        ctx: &mut Self,
        element: Mut<'_, Self::Element>,
    );
    fn message(
        view: &T,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

macro_rules! impl_orphan_view_for {
    ($ty: ty) => {
        impl<State, Action, Context> View<State, Action, Context> for $ty
        where
            Context: OrphanView<$ty, State, Action>,
        {
            type Element = Context::Element;

            type ViewState = Context::ViewState;

            fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
                Context::build(self, ctx)
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                Context::rebuild(self, prev, view_state, ctx, element)
            }

            fn teardown(
                &self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: Mut<'_, Self::Element>,
            ) {
                Context::teardown(self, view_state, ctx, element);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                Context::message(self, view_state, id_path, message, app_state)
            }
        }
    };
}

macro_rules! impl_as_orphan_view_for {
    ($ty: ty) => {
        impl<State, Action, Context> View<State, Action, Context> for $ty
        where
            Context: AsOrphanView<$ty, State, Action>,
        {
            type Element = <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>::Element;
            type ViewState = <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>::ViewState;

            fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
                <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>
                    ::build(&Context::as_view(self), ctx)
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>::rebuild(
                    &Context::as_view(self),
                    &Context::as_view(prev),
                    view_state,
                    ctx,
                    element,
                )
            }

            fn teardown(
                &self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: <Self::Element as ViewElement>::Mut<'_>,
            ) {
                <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>
                    ::teardown(&Context::as_view(self), view_state, ctx, element);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                <<Context as AsOrphanView<$ty, State, Action>>::V as View<State, Action, Context>>::message(
                    &Context::as_view(self),
                    view_state,
                    id_path,
                    message,
                    app_state,
                )
            }
        }
    };
}

// string impls
impl_as_orphan_view_for!(&'static str);
#[cfg(feature = "std")]
impl_orphan_view_for!(String);
#[cfg(feature = "std")]
impl_as_orphan_view_for!(std::borrow::Cow<'static, str>);
// Why does the following not work, but the `Cow` impl does??
// #[cfg(feature = "std")]
// impl_as_orphan_view_for!(std::sync::Arc<str>);

// number impls
impl_as_orphan_view_for!(f32);
impl_as_orphan_view_for!(f64);
impl_as_orphan_view_for!(i8);
impl_as_orphan_view_for!(u8);
impl_as_orphan_view_for!(i16);
impl_as_orphan_view_for!(u16);
impl_as_orphan_view_for!(i32);
impl_as_orphan_view_for!(u32);
impl_as_orphan_view_for!(i64);
impl_as_orphan_view_for!(u64);
impl_as_orphan_view_for!(u128);
impl_as_orphan_view_for!(isize);
impl_as_orphan_view_for!(usize);
