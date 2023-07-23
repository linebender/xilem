// Copyright 2023 the Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! generate_memoize_view {
    ($memoizeview:ident,
     $memoizestate:ident,
     $viewtrait:ident,
     $viewmarker:ty,
     $cx:ty,
     $changeflags:ty,
     $staticviewfunction:ident,
     $memoizeviewfunction:ident
    ) => {
        pub struct $memoizeview<D, F> {
            data: D,
            child_cb: F,
        }

        pub struct $memoizestate<T, A, V: $viewtrait<T, A>> {
            view: V,
            view_state: V::State,
            dirty: bool,
        }

        impl<D, V, F> $memoizeview<D, F>
        where
            F: Fn(&D) -> V,
        {
            pub fn new(data: D, child_cb: F) -> Self {
                $memoizeview { data, child_cb }
            }
        }

        impl<D, F> $viewmarker for $memoizeview<D, F> {}

        impl<T, A, D, V, F> $viewtrait<T, A> for $memoizeview<D, F>
        where
            D: PartialEq + Send + 'static,
            V: $viewtrait<T, A>,
            F: Fn(&D) -> V + Send,
        {
            type State = $memoizestate<T, A, V>;

            type Element = V::Element;

            fn build(&self, cx: &mut $cx) -> ($crate::Id, Self::State, Self::Element) {
                let view = (self.child_cb)(&self.data);
                let (id, view_state, element) = view.build(cx);
                let memoize_state = $memoizestate {
                    view,
                    view_state,
                    dirty: false,
                };
                (id, memoize_state, element)
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                id: &mut $crate::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> $changeflags {
                if std::mem::take(&mut state.dirty) || prev.data != self.data {
                    let view = (self.child_cb)(&self.data);
                    let changed = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
                    state.view = view;
                    changed
                } else {
                    <$changeflags>::empty()
                }
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                event: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                let r = state
                    .view
                    .message(id_path, &mut state.view_state, event, app_state);
                if matches!(r, $crate::MessageResult::RequestRebuild) {
                    state.dirty = true;
                }
                r
            }
        }

        /// A static view, all of the content of the `view` should be constant, as this function is only run once
        pub fn $staticviewfunction<V, F>(view: F) -> $memoizeview<(), impl Fn(&()) -> V>
        where
            F: Fn() -> V + 'static,
        {
            $memoizeview::new((), move |_: &()| view())
        }

        /// Memoize the view, until the `data` changes (in which case `view` is called again)
        pub fn $memoizeviewfunction<D, V, F>(data: D, view: F) -> $memoizeview<D, F>
        where
            F: Fn(&D) -> V,
        {
            $memoizeview::new(data, view)
        }
    };
}
