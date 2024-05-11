// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[doc(hidden)]
#[macro_export]
macro_rules! impl_view_tuple {
    ( $viewseq:ident, $elements_splice: ident, $pod:ty, $cx:ty, $changeflags:ty, $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: $viewseq<T, A> ),* > $viewseq<T, A> for ( $( $t, )* ) {
            type State = ( $( $t::State, )*);

            fn build(&self, cx: &mut $cx, elements: &mut dyn $elements_splice) -> Self::State {
                let b = ( $( self.$i.build(cx, elements), )* );
                let state = ( $( b.$i, )*);
                state
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut dyn $elements_splice,
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
    ($viewseq:ident, $view:ident, $viewmarker: ident, $elements_splice: ident, $bound:ident, $cx:ty, $changeflags:ty, $pod:ty; $( $ss:tt )* ) => {

        /// A temporary "splice" to add, update, delete and monitor elements in a sequence of elements.
        /// It is mainly intended for view sequences
        ///
        /// Usually it's backed by a collection (e.g. `Vec`) that holds all the (existing) elements.
        /// It sweeps over the element collection and does updates in place.
        /// Internally it works by having a pointer/index to the current/old element (0 at the beginning),
        /// and the pointer is incremented by basically all methods that mutate that sequence.
        pub trait $elements_splice {
            /// Insert a new element at the current index in the resulting collection (and increment the index by 1)
            fn push(&mut self, element: $pod, cx: &mut $cx);
            /// Mutate the next existing element, and add it to the resulting collection (and increment the index by 1)
            fn mutate(&mut self, cx: &mut $cx) -> &mut $pod;
            // TODO(#160) this could also track view id changes (old_id, new_id)
            /// Mark any changes done by `mutate` on the current element (this doesn't change the index)
            fn mark(&mut self, changeflags: $changeflags, cx: &mut $cx) -> $changeflags;
            /// Delete the next n existing elements (this doesn't change the index)
            fn delete(&mut self, n: usize, cx: &mut $cx);
            /// Current length of the elements collection
            fn len(&self) -> usize;
            // TODO(#160) add a skip method when it is necessary (e.g. relevant for immutable ViewSequences like ropes)
        }

        impl<'a, 'b> $elements_splice for $crate::VecSplice<'a, 'b, $pod> {
            fn push(&mut self, element: $pod, _cx: &mut $cx) {
                self.push(element);
            }

            fn mutate(&mut self, _cx: &mut $cx) -> &mut $pod
            {
                self.mutate()
            }

            fn mark(&mut self, changeflags: $changeflags, _cx: &mut $cx) -> $changeflags
            {
                self.last_mutated_mut().map(|pod| pod.mark(changeflags)).unwrap_or_default()
            }

            fn delete(&mut self, n: usize, _cx: &mut $cx) {
                self.delete(n)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }

        /// This trait represents a (possibly empty) sequence of views.
        ///
        /// It is up to the parent view how to lay out and display them.
        pub trait $viewseq<T, A = ()> $( $ss )* {
            /// Associated states for the views.
            type State $( $ss )*;

            /// Build the associated widgets and initialize all states.
            ///
            /// To be able to monitor changes (e.g. tree-structure tracking) rather than just adding elements,
            /// this takes an element splice as well (when it could be just a `Vec` otherwise)
            fn build(&self, cx: &mut $cx, elements: &mut dyn $elements_splice) -> Self::State;

            /// Update the associated widget.
            ///
            /// Returns `true` when anything has changed.
            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut dyn $elements_splice,
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

            fn build(&self, cx: &mut $cx, elements: &mut dyn $elements_splice) -> Self::State {
                let (id, state, pod) = cx.with_new_pod(|cx| <V as $view<T, A>>::build(self, cx));
                elements.push(pod, cx);
                (state, id)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut dyn $elements_splice,
            ) -> $changeflags {
                let pod = elements.mutate(cx);
                let flags = cx.with_pod(pod, |el, cx| {
                    <V as $view<T, A>>::rebuild(
                        self,
                        cx,
                        prev,
                        &mut state.1,
                        &mut state.0,
                        el,
                    )
                });
                elements.mark(flags, cx)
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

            fn build(&self, cx: &mut $cx, elements: &mut dyn $elements_splice) -> Self::State {
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
                elements: &mut dyn $elements_splice,
            ) -> $changeflags {
                match (self, &mut *state, prev) {
                    (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, elements),
                    (None, Some(seq_state), Some(prev)) => {
                        let count = prev.count(&seq_state);
                        elements.delete(count, cx);
                        *state = None;

                        <$changeflags>::tree_structure()
                    }
                    (Some(this), None, None) => {
                        *state = Some(this.build(cx, elements));

                        <$changeflags>::tree_structure()
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

        impl<T, A, VT: $viewseq<T, A>> $viewseq<T, A> for Vec<VT> {
            type State = Vec<VT::State>;

            fn build(&self, cx: &mut $cx, elements: &mut dyn $elements_splice) -> Self::State {
                self.iter().map(|child| child.build(cx, elements)).collect()
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut dyn $elements_splice,
            ) -> $changeflags {
                let mut changed = <$changeflags>::default();
                for ((child, child_prev), child_state) in self.iter().zip(prev).zip(state.iter_mut()) {
                    let el_changed = child.rebuild(cx, child_prev, child_state, elements);
                    changed |= el_changed;
                }
                let n = self.len();
                if n < prev.len() {
                    let n_delete = state
                        .splice(n.., [])
                        .enumerate()
                        .map(|(i, state)| prev[n + i].count(&state))
                        .sum();
                    elements.delete(n_delete, cx);
                    changed |= <$changeflags>::tree_structure();
                } else if n > prev.len() {
                    for i in prev.len()..n {
                        state.push(self[i].build(cx, elements));
                    }
                    changed |= <$changeflags>::tree_structure();
                }
                changed
            }

            fn count(&self, state: &Self::State) -> usize {
                self.iter().zip(state).map(|(child, child_state)|
                    child.count(child_state))
                    .sum()
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                let mut result = $crate::MessageResult::Stale(message);
                for (child, child_state) in self.iter().zip(state) {
                    if let $crate::MessageResult::Stale(message) = result {
                        result = child.message(id_path, child_state, message, app_state);
                    } else {
                        break;
                    }
                }
                result
            }
        }

        /// This trait marks a type a
        #[doc = concat!(stringify!($view), ".")]
        ///
        /// This trait is a workaround for Rust's orphan rules. It serves as a switch between
        /// default and custom
        #[doc = concat!("`", stringify!($viewseq), "`")]
        /// implementations. You can't implement
        #[doc = concat!("`", stringify!($viewseq), "`")]
        /// for types which also implement
        #[doc = concat!("`", stringify!($viewmarker), "`.")]
        pub trait $viewmarker {}

        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags, ;);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0; 0);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1; 0, 1);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2; 0, 1, 2);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3; 0, 1, 2, 3);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7; 0, 1, 2, 3, 4, 5, 6, 7);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7, V8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
        $crate::impl_view_tuple!($viewseq, $elements_splice, $pod, $cx, $changeflags,
            V0, V1, V2, V3, V4, V5, V6, V7, V8, V9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    };
}
