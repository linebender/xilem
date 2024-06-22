// TODO document everything, possibly different naming
#![allow(missing_docs)]
// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId, ViewPathTracker};

/// This trait provides a way to add [`View`] implementations for types that would be restricted otherwise by the orphan rules.
/// Every type that can be supported with this trait, needs a concrete `View` implementation as seen below.
pub trait OrphanView<T, State, Action>: ViewPathTracker + Sized {
    type OrphanElement: ViewElement;
    type OrphanViewState;

    fn orphan_build(view: &T, ctx: &mut Self) -> (Self::OrphanElement, Self::OrphanViewState);
    fn orphan_rebuild<'el>(
        new: &T,
        prev: &T,
        view_state: &mut Self::OrphanViewState,
        ctx: &mut Self,
        element: Mut<'el, Self::OrphanElement>,
    ) -> Mut<'el, Self::OrphanElement>;

    fn orphan_teardown(
        view: &T,
        view_state: &mut Self::OrphanViewState,
        ctx: &mut Self,
        element: Mut<'_, Self::OrphanElement>,
    );
    fn orphan_message(
        view: &T,
        view_state: &mut Self::OrphanViewState,
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
            type Element = Context::OrphanElement;

            type ViewState = Context::OrphanViewState;

            fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
                Context::orphan_build(self, ctx)
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                Context::orphan_rebuild(self, prev, view_state, ctx, element)
            }

            fn teardown(
                &self,
                view_state: &mut Self::ViewState,
                ctx: &mut Context,
                element: Mut<'_, Self::Element>,
            ) {
                Context::orphan_teardown(self, view_state, ctx, element);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                Context::orphan_message(self, view_state, id_path, message, app_state)
            }
        }
    };
}

// string impls
impl_orphan_view_for!(&'static str);
impl_orphan_view_for!(alloc::string::String);
impl_orphan_view_for!(alloc::borrow::Cow<'static, str>);

// number impls
impl_orphan_view_for!(f32);
impl_orphan_view_for!(f64);
impl_orphan_view_for!(i8);
impl_orphan_view_for!(u8);
impl_orphan_view_for!(i16);
impl_orphan_view_for!(u16);
impl_orphan_view_for!(i32);
impl_orphan_view_for!(u32);
impl_orphan_view_for!(i64);
impl_orphan_view_for!(u64);
impl_orphan_view_for!(u128);
impl_orphan_view_for!(isize);
impl_orphan_view_for!(usize);

#[cfg(feature = "kurbo")]
mod kurbo {
    use super::OrphanView;
    use crate::{DynMessage, MessageResult, Mut, View, ViewId};
    impl_orphan_view_for!(kurbo::PathSeg);
    impl_orphan_view_for!(kurbo::Arc);
    impl_orphan_view_for!(kurbo::BezPath);
    impl_orphan_view_for!(kurbo::Circle);
    impl_orphan_view_for!(kurbo::CircleSegment);
    impl_orphan_view_for!(kurbo::CubicBez);
    impl_orphan_view_for!(kurbo::Ellipse);
    impl_orphan_view_for!(kurbo::Line);
    impl_orphan_view_for!(kurbo::QuadBez);
    impl_orphan_view_for!(kurbo::Rect);
    impl_orphan_view_for!(kurbo::RoundedRect);
}
