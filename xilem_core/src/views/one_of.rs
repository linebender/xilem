// TODO document everything, possibly different naming
#![allow(missing_docs)]
use crate::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId, ViewPathTracker};

pub enum OneOf2<T1, T2> {
    A(T1),
    B(T2),
}

impl<T, E1: AsRef<T>, E2: AsRef<T>> AsRef<T> for OneOf2<E1, E2> {
    fn as_ref(&self) -> &T {
        match self {
            OneOf2::A(e) => <E1 as AsRef<T>>::as_ref(e),
            OneOf2::B(e) => <E2 as AsRef<T>>::as_ref(e),
        }
    }
}

pub trait OneOf2Ctx<E1: ViewElement, E2: ViewElement> {
    type OneOf2Element: ViewElement;

    fn upcast_one_of_2_element(elem: OneOf2<E1, E2>) -> Self::OneOf2Element;
    fn update_one_of_2_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOf2Element>,
        new_elem: OneOf2<E1, E2>,
    );
    fn rebuild_a<'a, State, Action, Context, V>(
        new: &V,
        prev: &V,
        view_state: &mut V::ViewState,
        ctx: &mut Context,
        elem: Mut<'a, Self::OneOf2Element>,
    ) -> Mut<'a, Self::OneOf2Element>
    where
        Context: ViewPathTracker,
        V: View<State, Action, Context, Element = E1>;

    fn rebuild_b<'a, State, Action, Context, V>(
        new: &V,
        prev: &V,
        view_state: &mut V::ViewState,
        ctx: &mut Context,
        elem: Mut<'a, Self::OneOf2Element>,
    ) -> Mut<'a, Self::OneOf2Element>
    where
        Context: ViewPathTracker,
        V: View<State, Action, Context, Element = E2>;

    fn teardown<State, Action, Context, V1, V2>(
        view: &OneOf2<V1, V2>,
        view_state: &mut OneOf2<V1::ViewState, V2::ViewState>,
        ctx: &mut Context,
        elem: &mut Mut<'_, Self::OneOf2Element>,
    ) where
        Context: ViewPathTracker,
        V1: View<State, Action, Context, Element = E1>,
        V2: View<State, Action, Context, Element = E2>;

    fn message<State, Action, Context, V1, V2>(
        view: &OneOf2<V1, V2>,
        view_state: &mut OneOf2<V1::ViewState, V2::ViewState>,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>
    where
        Context: ViewPathTracker,
        V1: View<State, Action, Context, Element = E1>,
        V2: View<State, Action, Context, Element = E2>;
}

impl<V1, V2, Context, T, A> View<T, A, Context> for OneOf2<V1, V2>
where
    T: 'static,
    A: 'static,
    Context: ViewPathTracker + OneOf2Ctx<V1::Element, V2::Element>,
    V1: View<T, A, Context>,
    V2: View<T, A, Context>,
{
    type Element = Context::OneOf2Element;

    type ViewState = OneOf2<V1::ViewState, V2::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        match self {
            OneOf2::A(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_2_element(OneOf2::A(element)),
                    OneOf2::A(state),
                )
            }
            OneOf2::B(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_2_element(OneOf2::B(element)),
                    OneOf2::B(state),
                )
            }
        }
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        match (prev, self, view_state) {
            (OneOf2::A(prev), OneOf2::A(new), state) => {
                let OneOf2::A(state) = state else {
                    unreachable!()
                };
                Context::rebuild_a(new, prev, state, ctx, element)
            }
            (OneOf2::B(prev), OneOf2::B(new), state) => {
                let OneOf2::B(state) = state else {
                    unreachable!()
                };
                Context::rebuild_b(new, prev, state, ctx, element)
            }
            (_, OneOf2::A(new), view_state) => {
                Context::teardown(prev, view_state, ctx, &mut element);
                let (new_element, state) = new.build(ctx);
                *view_state = OneOf2::A(state);
                Context::update_one_of_2_element_mut(&mut element, OneOf2::A(new_element));
                element
            }
            (_, OneOf2::B(new), view_state) => {
                Context::teardown(prev, view_state, ctx, &mut element);
                let (new_element, state) = new.build(ctx);
                *view_state = OneOf2::B(state);
                Context::update_one_of_2_element_mut(&mut element, OneOf2::B(new_element));
                element
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'_, Self::Element>,
    ) {
        Context::teardown(self, view_state, ctx, &mut element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut T,
    ) -> MessageResult<A> {
        Context::message(self, view_state, id_path, message, app_state)
    }
}
