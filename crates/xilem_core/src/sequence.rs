// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[doc(hidden)]
#[macro_export]
macro_rules! impl_view_tuple {
    ( $viewseq:ident, $pod:ty, $cx:ty, $changeflags:ty, $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: ViewSequence<T, A> ),* > ViewSequence<T, A> for ( $( $t, )* ) {
            type State = ( $( $t::State, )*);

            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State {
                let b = ( $( self.$i.build(cx, elements), )* );
                let state = ( $( b.$i, )*);
                state
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut $crate::VecSplice<$pod>,
            ) -> ChangeFlags {
                let mut changed = <$changeflags>::default();
                $(
                    let el_changed = self.$i.rebuild(cx, &prev.$i, &mut state.$i, els);
                    changed |= el_changed;
                )*
                changed
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                $crate::MessageResult::Stale(message)
                $(
                    .or(|message|{
                        self.$i.message(id_path, &mut state.$i, message, app_state)
                    })
                )*
            }

            fn count(&self, state: &Self::State) -> usize {
                0
                $(
                    + self.$i.count(&state.$i)
                )*
            }
        }
    }
}

#[macro_export]
macro_rules! generate_viewsequence_trait {
    ($viewseq:ident, $view:ident, $viewmarker: ident, $bound:ident, $cx:ty, $changeflags:ty, $pod:ty; $( $ss:tt )* ) => {
        pub trait $viewseq<T, A = ()> $( $ss )* {
            /// Associated states for the views.
            type State $( $ss )*;

            /// Build the associated widgets and initialize all states.
            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State;

            /// Update the associated widget.
            ///
            /// Returns `true` when anything has changed.
            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                element: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags;

            /// Propagate a message.
            ///
            /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
            /// of ids beginning at an element of this view_sequence.
            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A>;

            /// Returns the current amount of widgets built by this sequence.
            fn count(&self, state: &Self::State) -> usize;
        }

        impl<T, A, V: $view<T, A> + $viewmarker> $viewseq<T, A> for V
        where
            V::Element: $bound + 'static,
        {
            type State = (<V as $view<T, A>>::State, $crate::Id);

            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State {
                let (id, state, element) = <V as $view<T, A>>::build(self, cx);
                elements.push(<$pod>::new(element));
                (state, id)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                element: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags {
                let el = element.mutate();
                let downcast = el.downcast_mut().unwrap();
                let flags = <V as $view<T, A>>::rebuild(
                    self,
                    cx,
                    prev,
                    &mut state.1,
                    &mut state.0,
                    downcast,
                );

                el.mark(flags)
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                if let Some((first, rest_path)) = id_path.split_first() {
                    if first == &state.1 {
                        return <V as $view<T, A>>::message(
                            self,
                            rest_path,
                            &mut state.0,
                            message,
                            app_state,
                        );
                    }
                }
                $crate::MessageResult::Stale(message)
            }

            fn count(&self, _state: &Self::State) -> usize {
                1
            }
        }

        impl<T, A, VT: $viewseq<T, A>> $viewseq<T, A> for Option<VT> {
            type State = Option<VT::State>;

            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State {
                match self {
                    None => None,
                    Some(vt) => {
                        let state = vt.build(cx, elements);
                        Some(state)
                    }
                }
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                element: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags {
                match (self, &mut *state, prev) {
                    (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, element),
                    (None, Some(seq_state), Some(prev)) => {
                        let count = prev.count(&seq_state);
                        element.delete(count);
                        *state = None;

                        <$changeflags>::all()
                    }
                    (Some(this), None, None) => {
                        let seq_state = element.as_vec(|vec| this.build(cx, vec));
                        *state = Some(seq_state);

                        <$changeflags>::all()
                    }
                    (None, None, None) => <$changeflags>::empty(),
                    _ => panic!("non matching state and prev value"),
                }
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                match (self, state) {
                    (Some(vt), Some(state)) => vt.message(id_path, state, message, app_state),
                    (None, None) => $crate::MessageResult::Stale(message),
                    _ => panic!("non matching state and prev value"),
                }
            }

            fn count(&self, state: &Self::State) -> usize {
                match (self, state) {
                    (Some(vt), Some(state)) => vt.count(state),
                    (None, None) => 0,
                    _ => panic!("non matching state and prev value"),
                }
            }
        }

        /// This trait marks a type a
        #[doc = concat!(stringify!($view), ".")]
        ///
        /// This trait is a workaround for Rust's orphan rules. It serves as a switch between default"]
        /// and custom
        #[doc = concat!("`", stringify!($viewseq), "`")]
        /// implementations. You can't implement
        #[doc = concat!("`", stringify!($viewseq), "`")]
        /// for types which also implement
        #[doc = concat!("`", stringify!($viewmarker), "`.")]
        pub trait $viewmarker {}

        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0; 0);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1; 0, 1);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2; 0, 1, 2);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3; 0, 1, 2, 3);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7; 0, 1, 2, 3, 4, 5, 6, 7);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7, V8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7, V8, V9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    };
}
