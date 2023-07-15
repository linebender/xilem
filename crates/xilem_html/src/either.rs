use wasm_bindgen::throw_str;

use crate::{ChangeFlags, Cx, Pod, View, ViewMarker, ViewSequence};

/// This view container can switch between two views.
///
/// It is a statically-typed alternative to the type-erased `AnyView`.
pub enum Either<T1, T2> {
    Left(T1),
    Right(T2),
}

impl<E1, E2> AsRef<web_sys::Node> for Either<E1, E2>
where
    E1: AsRef<web_sys::Node>,
    E2: AsRef<web_sys::Node>,
{
    fn as_ref(&self) -> &web_sys::Node {
        match self {
            Either::Left(view) => view.as_ref(),
            Either::Right(view) => view.as_ref(),
        }
    }
}

impl<T, A, V1, V2> View<T, A> for Either<V1, V2>
where
    V1: View<T, A> + ViewMarker,
    V2: View<T, A> + ViewMarker,
    V1::Element: AsRef<web_sys::Node> + 'static,
    V2::Element: AsRef<web_sys::Node> + 'static,
{
    type State = Either<V1::State, V2::State>;
    type Element = Either<V1::Element, V2::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        match self {
            Either::Left(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::Left(state), Either::Left(el))
            }
            Either::Right(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::Right(state), Either::Right(el))
            }
        }
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        match (prev, self) {
            (Either::Left(_), Either::Right(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::Right(new_state);
                *element = Either::Right(new_element);
                ChangeFlags::STRUCTURE
            }
            (Either::Right(_), Either::Left(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::Left(new_state);
                *element = Either::Left(new_element);
                ChangeFlags::STRUCTURE
            }
            (Either::Left(prev_view), Either::Left(view)) => {
                let (Either::Left(state), Either::Left(element)) = (state, element) else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                // Cannot do mutable casting, so take ownership of state.
                view.rebuild(cx, prev_view, id, state, element)
            }
            (Either::Right(prev_view), Either::Right(view)) => {
                let (Either::Right(state), Either::Right(element)) = (state, element) else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                // Cannot do mutable casting, so take ownership of state.
                view.rebuild(cx, prev_view, id, state, element)
            }
        }
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match self {
            Either::Left(view) => {
                let Either::Left(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
            Either::Right(view) => {
                let Either::Right(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
        }
    }
}

impl<T, A, V1, V2> ViewSequence<T, A> for Either<V1, V2>
where
    V1: ViewSequence<T, A>,
    V2: ViewSequence<T, A>,
{
    type State = Either<V1::State, V2::State>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
        match self {
            Either::Left(view_sequence) => Either::Left(view_sequence.build(cx, elements)),
            Either::Right(view_sequence) => Either::Right(view_sequence.build(cx, elements)),
        }
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut xilem_core::VecSplice<Pod>,
    ) -> ChangeFlags {
        match (prev, self) {
            (Either::Left(_), Either::Right(view_sequence)) => {
                let new_state = element.as_vec(|elements| view_sequence.build(cx, elements));
                *state = Either::Right(new_state);
                ChangeFlags::STRUCTURE
            }
            (Either::Right(_), Either::Left(view_sequence)) => {
                let new_state = element.as_vec(|elements| view_sequence.build(cx, elements));
                *state = Either::Left(new_state);
                ChangeFlags::STRUCTURE
            }
            (Either::Left(prev_view), Either::Left(view_sequence)) => {
                let Either::Left(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.rebuild(cx, prev_view, state, element)
            }
            (Either::Right(prev_view), Either::Right(view_sequence)) => {
                let Either::Right(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.rebuild(cx, prev_view, state, element)
            }
        }
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match self {
            Either::Left(view_sequence) => {
                let Either::Left(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.message(id_path, state, message, app_state)
            }
            Either::Right(view_sequence) => {
                let Either::Right(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.message(id_path, state, message, app_state)
            }
        }
    }

    fn count(&self, state: &Self::State) -> usize {
        match self {
            Either::Left(view_sequence) => {
                let Either::Left(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.count(state)
            }
            Either::Right(view_sequence) => {
                let Either::Right(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.count(state)
            }
        }
    }
}
