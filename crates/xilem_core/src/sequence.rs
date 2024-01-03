// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[doc(hidden)]
#[macro_export]
macro_rules! impl_view_tuple {
    ( $viewseq:ident, $pod:ty, $cx:ty, $changeflags:ty, $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: $viewseq<T, A> ),* > $viewseq<T, A> for ( $( $t, )* ) {
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
    ($viewseq:ident, $anyviewseq:ident, $boxedviewseq:ident, $view:ident, $viewmarker: ident, $bound:ident, $cx:ty, $changeflags:ty, $pod:ty; ($( $ss:tt )*); ($( $ss_bounds:tt )*) ) => {
        /// This trait represents a (possibly empty) sequence of views.
        ///
        /// It is up to the parent view how to lay out and display them.
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
                elements: &mut $crate::VecSplice<$pod>,
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

        /// A trait enabling type erasure of view sequences.
        pub trait $anyviewseq<T, A = ()> $( $ss )* {
            fn as_any(&self) -> &dyn std::any::Any;

            /// Build the associated widgets and initialize all states.
            fn dyn_build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Box<dyn std::any::Any $( $ss_bounds )* >;

            /// Update the associated widget.
            ///
            /// Returns `true` when anything has changed.
            fn dyn_rebuild(
                &self,
                cx: &mut $cx,
                prev: &dyn $anyviewseq<T, A>,
                state: &mut Box<dyn std::any::Any $( $ss_bounds )* >,
                elements: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags;

            /// Propagate a message.
            ///
            /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
            /// of ids beginning at an element of this view_sequence.
            fn dyn_message(
                &self,
                id_path: &[$crate::Id],
                state: &mut dyn std::any::Any,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A>;

            /// Returns the current amount of widgets built by this sequence.
            fn dyn_count(&self, state: &dyn std::any::Any) -> usize;
        }


        impl<T, A, VS: $viewseq<T, A> + 'static> $anyviewseq<T, A> for VS
        where
            VS::State: 'static,
        {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn dyn_build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Box<dyn std::any::Any $( $ss_bounds )* > {
                Box::new(self.build(cx, elements))
            }

            fn dyn_rebuild(
                &self,
                cx: &mut $cx,
                prev: &dyn $anyviewseq<T, A>,
                state: &mut Box<dyn std::any::Any $( $ss_bounds )* >,
                elements: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags
            {
                use std::ops::DerefMut;
                if let Some(prev) = prev.as_any().downcast_ref() {
                    if let Some(state) = state.downcast_mut() {
                        self.rebuild(cx, prev, state, elements)
                    } else {
                        eprintln!("downcast of state failed in dyn_rebuild");
                        <$changeflags>::default()
                    }
                } else {
                    let new_state = elements.as_vec(|vec| self.build(cx, vec));
                    *state = Box::new(new_state);
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

            fn dyn_count(&self, state: &dyn std::any::Any) -> usize {
                if let Some(state) = state.downcast_ref() {
                    self.count(state)
                } else {
                    // Possibly softer failure?
                    panic!("downcast error in dyn_count");
                }
            }
        }

        pub type $boxedviewseq<T, A = ()> = Box<dyn $anyviewseq<T, A> $( $ss_bounds )* >;

        impl<T, A> $viewseq<T, A> for $boxedviewseq<T, A> {
            type State = Box<dyn std::any::Any $( $ss_bounds )* >;

            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State {
                use std::ops::Deref;
                self.deref().dyn_build(cx, elements)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags {
                use std::ops::Deref;
                self.deref().dyn_rebuild(cx, prev.deref(), state, elements)
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

            fn count(&self, state: &Self::State) -> usize {
                use std::ops::Deref;
                self.deref().dyn_count(state.deref())
            }
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
                elements: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags {
                let el = elements.mutate();
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
                elements: &mut $crate::VecSplice<$pod>,
            ) -> $changeflags {
                match (self, &mut *state, prev) {
                    (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, elements),
                    (None, Some(seq_state), Some(prev)) => {
                        let count = prev.count(&seq_state);
                        elements.delete(count);
                        *state = None;

                        <$changeflags>::tree_structure()
                    }
                    (Some(this), None, None) => {
                        let seq_state = elements.as_vec(|vec| this.build(cx, vec));
                        *state = Some(seq_state);

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

            fn build(&self, cx: &mut $cx, elements: &mut Vec<$pod>) -> Self::State {
                self.iter().map(|child| child.build(cx, elements)).collect()
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut $crate::VecSplice<$pod>,
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
                    elements.delete(n_delete);
                    changed |= <$changeflags>::tree_structure();
                } else if n > prev.len() {
                    let mut child_elements = vec![];
                    for i in prev.len()..n {
                        state.push(self[i].build(cx, &mut child_elements));
                    }
                    // Discussion question: should VecSplice get an extend method?
                    for element in child_elements {
                        elements.push(element);
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

        $crate::impl_view_tuple!($viewseq, $pod, $cx, $changeflags, ;);
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
