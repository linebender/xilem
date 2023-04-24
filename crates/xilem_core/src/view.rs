// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_view_trait {
    ($viewtrait:ident, $bound:ident, $cx:ty, $changeflags: ty) => {
        /// A view object representing a node in the UI.
        ///
        /// This is a central trait for representing UI. An app will generate a tree of
        /// these objects (the view tree) as the primary interface for expressing UI.
        /// The view tree is transitory and is retained only long enough to dispatch
        /// events and then serve as a reference for diffing for the next view tree.
        ///
        /// The framework will then run methods on these views to create the associated
        /// state tree and element tree, as well as incremental updates and event
        /// propagation.
        ///
        /// The
        #[doc = concat!("`", stringify!($viewtrait), "`")]
        // trait is parameterized by `T`, which is known as the "app state",
        /// and also a type for actions which are passed up the tree in event
        /// propagation. During event handling, mutable access to the app state is
        /// given to view nodes, which in turn can expose it to callbacks.
        pub trait $viewtrait<T, A = ()>: Send {
            /// Associated state for the view.
            type State: Send;

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
    };
}
