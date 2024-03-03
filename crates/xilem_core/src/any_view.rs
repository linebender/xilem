// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_anyview_trait {
    ($anyview:ident, $viewtrait:ident, $viewmarker:ty, $cx:ty, $changeflags:ty, $anywidget:ident, $boxedview:ident; $($ss:tt)*) => {
        /// A trait enabling type erasure of views.
        pub trait $anyview<T, A = ()> {
            fn as_any(&self) -> &dyn std::any::Any;

            fn dyn_build(
                &self,
                cx: &mut $cx,
            ) -> ($crate::Id, Box<dyn std::any::Any $( $ss )* >, Box<dyn $anywidget>);

            fn dyn_rebuild(
                &self,
                cx: &mut $cx,
                prev: &dyn $anyview<T, A>,
                id: &mut $crate::Id,
                state: &mut Box<dyn std::any::Any $( $ss )* >,
                element: &mut Box<dyn $anywidget>,
            ) -> $changeflags;

            fn dyn_message(
                &self,
                id_path: &[$crate::Id],
                state: &mut dyn std::any::Any,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A>;
        }

        impl<T, A, V: $viewtrait<T, A> + 'static> $anyview<T, A> for V
        where
            V::State: 'static,
            V::Element: $anywidget + 'static,
        {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn dyn_build(
                &self,
                cx: &mut $cx,
            ) -> ($crate::Id, Box<dyn std::any::Any $( $ss )* >, Box<dyn $anywidget>) {
                let (id, state, element) = self.build(cx);
                (id, Box::new(state), Box::new(element))
            }

            fn dyn_rebuild(
                &self,
                cx: &mut $cx,
                prev: &dyn $anyview<T, A>,
                id: &mut $crate::Id,
                state: &mut Box<dyn std::any::Any $( $ss )* >,
                element: &mut Box<dyn $anywidget>,
            ) -> ChangeFlags {
                use std::ops::DerefMut;
                if let Some(prev) = prev.as_any().downcast_ref() {
                    if let Some(state) = state.downcast_mut() {
                        if let Some(element) = element.deref_mut().as_any_mut().downcast_mut() {
                            self.rebuild(cx, prev, id, state, element)
                        } else {
                            eprintln!("downcast of element failed in dyn_rebuild");
                            <$changeflags>::default()
                        }
                    } else {
                        eprintln!("downcast of state failed in dyn_rebuild");
                        <$changeflags>::default()
                    }
                } else {
                    cx.delete_descendants();
                    let (new_id, new_state, new_element) = self.build(cx);
                    *id = new_id;
                    *state = Box::new(new_state);
                    *element = Box::new(new_element);
                    <$changeflags>::tree_structure()
                }
            }

            fn dyn_message(
                &self,
                id_path: &[$crate::Id],
                state: &mut dyn std::any::Any,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                if let Some(state) = state.downcast_mut() {
                    self.message(id_path, state, message, app_state)
                } else {
                    // Possibly softer failure?
                    panic!("downcast error in dyn_event");
                }
            }
        }

        pub type $boxedview<T, A = ()> = Box<dyn $anyview<T, A> $( $ss )* >;

        impl<T, A> $viewmarker for $boxedview<T, A> {}

        impl<T, A> $viewtrait<T, A> for $boxedview<T, A> {
            type State = Box<dyn std::any::Any $( $ss )* >;

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
                use std::ops::Deref;
                self.deref()
                    .dyn_rebuild(cx, prev.deref(), id, state, element)
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
