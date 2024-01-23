// Copyright 2024 the Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_rc_view {
    ($($rc_ty:ident)::*, $viewtrait:ident, $viewmarker:ty, $cx:ty, $changeflags:ty; $($ss:tt)*) => {
        impl<V> $viewmarker for $($rc_ty)::*<V> {}

        impl<T, A, V: $viewtrait<T, A> $( $ss )*> $viewtrait<T, A> for $($rc_ty)::*<V> {
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
    };
}
