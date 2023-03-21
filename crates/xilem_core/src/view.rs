// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

/// Create the `View` trait for a particular xilem context (e.g. html, native, ...).
///
/// Arguments are
///
///  - `$viewtrait` - The name of the view trait we want to generate.
///  - `$bound` - A bound on all element types that will be used.
///  - `$cx` - The name of text context type that will be passed to the `build`/`rebuild`
///    methods, and be responsible for managing element creation & deletion.
///  - `$changeflags` - The type that reports down/up the tree. Can be used to avoid
///    doing work when we can prove nothing needs doing.
///  - `$ss` - (optional) parent traits to this trait (e.g. `:Send`). Also applied to
///    the state type requirements
#[macro_export]
macro_rules! generate_view_trait {
    ($viewtrait:ident, $bound:ident, $cx:ty, $changeflags:ty; $($ss:tt)*) => {
        /// A view object representing a node in the UI.
        ///
        /// This is a central trait for representing UI. An app will generate a tree of
        /// these objects (the view tree) as the primary interface for expressing UI.
        /// The view tree is transitory and is retained only long enough to dispatch
        /// messages and then serve as a reference for diffing for the next view tree.
        ///
        /// The framework will then run methods on these views to create the associated
        /// state tree and element tree, as well as incremental updates and message
        /// propagation.
        ///
        /// The
        #[doc = concat!("`", stringify!($viewtrait), "`")]
        // trait is parameterized by `T`, which is known as the "app state",
        /// and also a type for actions which are passed up the tree in message
        /// propagation. During message handling, mutable access to the app state is
        /// given to view nodes, which in turn can expose it to callbacks.
        pub trait $viewtrait<T, A = ()> $( $ss )* {
            /// Associated state for the view.
            type State $( $ss )*;

            /// The associated element for the view.
            type Element: $bound;

            /// Build the associated widget and initialize state.
            fn build(&self, cx: &mut $cx) -> ($crate::Id, Self::State, Self::Element);

            /// Update the associated element.
            ///
            /// Returns an indication of what, if anything, has changed.
            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                id: &mut $crate::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> $changeflags;

            /// Propagate a message.
            ///
            /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
            /// of ids beginning at a child of this view.
            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A>;
        }

        pub struct Adapt<OutData, OutMsg, InData, InMsg, F: Fn(&mut OutData, AdaptThunk<InData, InMsg, V>) -> $crate::MessageResult<OutMsg>, V: View<InData, InMsg>> {
            f: F,
            child: V,
            phantom: std::marker::PhantomData<fn() -> (OutData, OutMsg, InData, InMsg)>,
        }

        /// A "thunk" which dispatches an message to an adapt node's child.
        ///
        /// The closure passed to [`Adapt`][crate::Adapt] should call this thunk with the child's
        /// app state.
        pub struct AdaptThunk<'a, InData, InMsg, V: View<InData, InMsg>> {
            child: &'a V,
            state: &'a mut V::State,
            id_path: &'a [$crate::Id],
            message: Box<dyn std::any::Any>,
        }

        impl<OutData, OutMsg, InData, InMsg, F: Fn(&mut OutData, AdaptThunk<InData, InMsg, V>) -> $crate::MessageResult<OutMsg>, V: View<InData, InMsg>>
            Adapt<OutData, OutMsg, InData, InMsg, F, V>
        {
            pub fn new(f: F, child: V) -> Self {
                Adapt {
                    f,
                    child,
                    phantom: Default::default(),
                }
            }
        }

        impl<'a, InData, InMsg, V: View<InData, InMsg>> AdaptThunk<'a, InData, InMsg, V> {
            pub fn call(self, app_state: &mut InData) -> $crate::MessageResult<InMsg> {
                self.child
                    .message(self.id_path, self.state, self.message, app_state)
            }
        }

        impl<OutData, OutMsg, InData, InMsg, F: Fn(&mut OutData, AdaptThunk<InData, InMsg, V>) -> $crate::MessageResult<OutMsg> + Send, V: View<InData, InMsg>>
            View<OutData, OutMsg> for Adapt<OutData, OutMsg, InData, InMsg, F, V>
        {
            type State = V::State;

            type Element = V::Element;

            fn build(&self, cx: &mut Cx) -> ($crate::Id, Self::State, Self::Element) {
                self.child.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                id: &mut $crate::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> $changeflags {
                self.child.rebuild(cx, &prev.child, id, state, element)
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut OutData,
            ) -> $crate::MessageResult<OutMsg> {
                let thunk = AdaptThunk {
                    child: &self.child,
                    state,
                    id_path,
                    message,
                };
                (self.f)(app_state, thunk)
            }
        }

        impl<OutData, OutMsg, InData, InMsg, F: Fn(&mut OutData, AdaptThunk<InData, InMsg, V>) -> $crate::MessageResult<OutMsg>, V: View<InData, InMsg>>
        ViewMarker for Adapt<OutData, OutMsg, InData, InMsg, F, V> {}

    };
}
