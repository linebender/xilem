use wasm_bindgen::throw_str;

use crate::{ChangeFlags, Cx, Pod, View, ViewMarker, ViewSequence};

/// This view container can switch between two views.
///
/// It is a statically-typed alternative to the type-erased `AnyView`.
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B> AsRef<web_sys::Node> for Either<A, B>
where
    A: AsRef<web_sys::Node>,
    B: AsRef<web_sys::Node>,
{
    fn as_ref(&self) -> &web_sys::Node {
        match self {
            Either::A(view) => view.as_ref(),
            Either::B(view) => view.as_ref(),
        }
    }
}

impl<VT, VA, A, B> View<VT, VA> for Either<A, B>
where
    A: View<VT, VA> + ViewMarker,
    B: View<VT, VA> + ViewMarker,
    A::Element: AsRef<web_sys::Node> + 'static,
    B::Element: AsRef<web_sys::Node> + 'static,
{
    type State = Either<A::State, B::State>;
    type Element = Either<A::Element, B::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        match self {
            Either::A(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::A(state), Either::A(el))
            }
            Either::B(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::B(state), Either::B(el))
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
            (Either::A(_), Either::B(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::B(new_state);
                *element = Either::B(new_element);
                ChangeFlags::STRUCTURE
            }
            (Either::B(_), Either::A(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::A(new_state);
                *element = Either::A(new_element);
                ChangeFlags::STRUCTURE
            }
            (Either::A(prev_view), Either::A(view)) => {
                let (Either::A(state), Either::A(element)) = (state, element) else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                // Cannot do mutable casting, so take ownership of state.
                view.rebuild(cx, prev_view, id, state, element)
            }
            (Either::B(prev_view), Either::B(view)) => {
                let (Either::B(state), Either::B(element)) = (state, element) else {
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
        app_state: &mut VT,
    ) -> xilem_core::MessageResult<VA> {
        match self {
            Either::A(view) => {
                let Either::A(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
            Either::B(view) => {
                let Either::B(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
        }
    }
}

impl<VT, VA, A, B> ViewSequence<VT, VA> for Either<A, B>
where
    A: ViewSequence<VT, VA>,
    B: ViewSequence<VT, VA>,
{
    type State = Either<A::State, B::State>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
        match self {
            Either::A(view_sequence) => Either::A(view_sequence.build(cx, elements)),
            Either::B(view_sequence) => Either::B(view_sequence.build(cx, elements)),
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
            (Either::A(_), Either::B(view_sequence)) => {
                let new_state = element.as_vec(|elements| view_sequence.build(cx, elements));
                *state = Either::B(new_state);
                ChangeFlags::STRUCTURE
            }
            (Either::B(_), Either::A(view_sequence)) => {
                let new_state = element.as_vec(|elements| view_sequence.build(cx, elements));
                *state = Either::A(new_state);
                ChangeFlags::STRUCTURE
            }
            (Either::A(prev_view), Either::A(view_sequence)) => {
                let Either::A(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.rebuild(cx, prev_view, state, element)
            }
            (Either::B(prev_view), Either::B(view_sequence)) => {
                let Either::B(state) = state else {
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
        app_state: &mut VT,
    ) -> xilem_core::MessageResult<VA> {
        match self {
            Either::A(view_sequence) => {
                let Either::A(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.message(id_path, state, message, app_state)
            }
            Either::B(view_sequence) => {
                let Either::B(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.message(id_path, state, message, app_state)
            }
        }
    }

    fn count(&self, state: &Self::State) -> usize {
        match self {
            Either::A(view_sequence) => {
                let Either::A(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.count(state)
            }
            Either::B(view_sequence) => {
                let Either::B(state) = state else {
                    throw_str("invalid state/view_sequence in Either (unreachable)");
                };
                view_sequence.count(state)
            }
        }
    }
}
