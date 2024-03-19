// Copyright 2024 the Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_rc_view {
    ($($rc_ty:ident)::*, $viewtrait:ident, $viewmarker:ty, $cx:ty, $changeflags:ty, $anyview:ident, $anywidget:ident; $($state_bounds:tt)*) => {
        impl<V> $viewmarker for $($rc_ty)::*<V> {}

        impl<T, A, V: $viewtrait<T, A>> $viewtrait<T, A> for $($rc_ty)::*<V> {
            type State = V::State;

            type Element = V::Element;

            fn build(&self, cx: &mut Cx) -> ($crate::Id, Self::State, Self::Element) {
                V::build(self, cx)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                id: &mut $crate::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                if !$($rc_ty)::*::ptr_eq(self, prev) {
                    V::rebuild(self, cx, prev, id, state, element)
                } else {
                    ChangeFlags::empty()
                }
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                V::message(self, id_path, state, message, app_state)
            }
        }

        impl<T, A> $viewmarker for $($rc_ty)::*<dyn $anyview<T, A>> {}
        impl<T, A> $viewtrait<T, A> for $($rc_ty)::*<dyn $anyview<T, A>> {
            type State = Box<dyn std::any::Any + $( $state_bounds )*>;

            type Element = Box<dyn $anywidget>;

            fn build(&self, cx: &mut $cx) -> ($crate::Id, Self::State, Self::Element) {
                use std::ops::Deref;
                self.deref().dyn_build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                id: &mut $crate::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> $changeflags {
                if !$($rc_ty)::*::ptr_eq(self, prev) {
                    use std::ops::Deref;
                    self.deref()
                        .dyn_rebuild(cx, prev.deref(), id, state, element)
                } else {
                    ChangeFlags::empty()
                }

            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                use std::ops::{Deref, DerefMut};
                self.deref()
                    .dyn_message(id_path, state.deref_mut(), message, app_state)
            }
        }
    };
}
