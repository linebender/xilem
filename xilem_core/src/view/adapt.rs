// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_adapt_view {
    ($viewtrait:ident, $cx:ty, $changeflags:ty; $($ss:tt)*) => {
        /// A view that wraps a child view and modifies the state that callbacks have access to.
        ///
        /// # Examples
        ///
        /// Suppose you have an outer type that looks like
        ///
        /// ```ignore
        /// struct State {
        ///     todos: Vec<Todo>
        /// }
        /// ```
        ///
        /// and an inner type/view that looks like
        ///
        /// ```ignore
        /// struct Todo {
        ///     label: String
        /// }
        ///
        /// struct TodoView {
        ///     label: String
        /// }
        ///
        /// enum TodoAction {
        ///     Delete
        /// }
        ///
        /// impl View<Todo, TodoAction> for TodoView {
        ///     // ...
        /// }
        /// ```
        ///
        /// then your top-level action (`()`) and state type (`State`) don't match `TodoView`'s.
        /// You can use the `Adapt` view to mediate between them:
        ///
        /// ```ignore
        /// state
        ///     .todos
        ///     .enumerate()
        ///     .map(|(idx, todo)| {
        ///         Adapt::new(
        ///             move |data: &mut AppState, thunk| {
        ///                 if let MessageResult::Action(action) = thunk.call(&mut data.todos[idx]) {
        ///                     match action {
        ///                         TodoAction::Delete => data.todos.remove(idx),
        ///                     }
        ///                 }
        ///                 MessageResult::Nop
        ///             },
        ///             TodoView { label: todo.label }
        ///         )
        ///     })
        /// ```
        pub struct Adapt<
            ParentT,
            ParentA,
            ChildT,
            ChildA,
            V,
            F = fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> $crate::MessageResult<ParentA>,
        > {
            f: F,
            child: V,
            phantom: std::marker::PhantomData<fn() -> (ParentT, ParentA, ChildT, ChildA)>,
        }

        /// A "thunk" which dispatches an message to an adapt node's child.
        ///
        /// The closure passed to [`Adapt`][crate::Adapt] should call this thunk with the child's
        /// app state.
        pub struct AdaptThunk<'a, ChildT, ChildA, V: $viewtrait<ChildT, ChildA>> {
            child: &'a V,
            state: &'a mut V::State,
            id_path: &'a [$crate::Id],
            message: Box<dyn std::any::Any>,
        }

        impl<ParentT, ParentA, ChildT, ChildA, V, F> Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
        where
            V: $viewtrait<ChildT, ChildA>,
            F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> $crate::MessageResult<ParentA> $( $ss )*,
        {
            pub fn new(f: F, child: V) -> Self {
                Adapt {
                    f,
                    child,
                    phantom: Default::default(),
                }
            }
        }

        impl<'a, ChildT, ChildA, V: $viewtrait<ChildT, ChildA>> AdaptThunk<'a, ChildT, ChildA, V> {
            pub fn call(self, app_state: &mut ChildT) -> $crate::MessageResult<ChildA> {
                self.child
                    .message(self.id_path, self.state, self.message, app_state)
            }
        }

        impl<ParentT, ParentA, ChildT, ChildA, V, F> $viewtrait<ParentT, ParentA>
            for Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
        where
            V: $viewtrait<ChildT, ChildA>,
            F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> $crate::MessageResult<ParentA> $( $ss )*,
        {
            type State = V::State;

            type Element = V::Element;

            fn build(&self, cx: &mut $cx) -> ($crate::Id, Self::State, Self::Element) {
                self.child.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
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
                app_state: &mut ParentT,
            ) -> $crate::MessageResult<ParentA> {
                let thunk = AdaptThunk {
                    child: &self.child,
                    state,
                    id_path,
                    message,
                };
                (self.f)(app_state, thunk)
            }
        }

        impl<ParentT, ParentA, ChildT, ChildA, V, F> ViewMarker
            for Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
        where
            V: $viewtrait<ChildT, ChildA>,
            F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> $crate::MessageResult<ParentA> $( $ss )*,
        {
        }
    };
}

#[macro_export]
macro_rules! generate_adapt_state_view {
    ($viewtrait:ident, $cx:ty, $changeflags:ty; $($ss:tt)*) => {
        /// A view that wraps a child view and modifies the state that callbacks have access to.
        pub struct AdaptState<ParentT, ChildT, V, F = fn(&mut ParentT) -> &mut ChildT> {
            f: F,
            child: V,
            phantom: std::marker::PhantomData<fn() -> (ParentT, ChildT)>,
        }

        impl<ParentT, ChildT, V, F> AdaptState<ParentT, ChildT, V, F>
        where
            F: Fn(&mut ParentT) -> &mut ChildT $( $ss )*,
        {
            pub fn new(f: F, child: V) -> Self {
                Self {
                    f,
                    child,
                    phantom: Default::default(),
                }
            }
        }

        impl<ParentT, ChildT, A, V, F> $viewtrait<ParentT, A> for AdaptState<ParentT, ChildT, V, F>
        where
            V: $viewtrait<ChildT, A>,
            F: Fn(&mut ParentT) -> &mut ChildT $( $ss )*,
        {
            type State = V::State;
            type Element = V::Element;

            fn build(&self, cx: &mut $cx) -> ($crate::Id, Self::State, Self::Element) {
                self.child.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
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
                app_state: &mut ParentT,
            ) -> $crate::MessageResult<A> {
                self.child
                    .message(id_path, state, message, (self.f)(app_state))
            }
        }

        impl<ParentT, ChildT, V, F> ViewMarker for AdaptState<ParentT, ChildT, V, F> where
            F: Fn(&mut ParentT) -> &mut ChildT $( $ss )*
        {
        }
    };
}
