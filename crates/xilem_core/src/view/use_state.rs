#[macro_export]
macro_rules! generate_use_state_view {
    ($viewtrait:ident, $cx:ty, $changeflags:ty; $($ss:tt)*) => {
        pub fn use_state<F, S, G, V>(make_state: F, make_view: G) -> UseState<F, S, G, V> {
            UseState::new(make_state, make_view)
        }

        pub struct UseState<F, S, G, V> {
            make_state: F,
            state: Option<S>,
            make_view: G,
            view: Option<V>,
        }

        impl<F, S, G, V> UseState<F, S, G, V> {
            pub fn new(make_state: F, make_view: G) -> Self {
                Self {
                    make_state,
                    state: None,
                    make_view,
                    view: None,
                }
            }
        }

        impl<T, A, F, S, G, V> View<T, A> for UseState<F, S, G, V>
        where
            S: Send,
            F: FnMut() -> S + Send,
            G: FnMut(&mut S) -> V + Send,
            V: $viewtrait<S, A>,
        {
            type State = V::State;
            type Element = V::Element;

            fn build(&self, cx: &mut $cx) -> (Id, Self::State, Self::Element) {
                todo!()
            }

            fn rebuild(
                &self,
                cx: &mut $cx,
                prev: &Self,
                id: &mut Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> $changeflags {
               todo!()
            }

            fn message(
                &self,
                id_path: &[$crate::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> $crate::MessageResult<A> {
                todo!()
            }
        }
    };
}
