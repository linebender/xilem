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
    type OneOfTwoElement: ViewElement;

    fn with_downcast_a<R>(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        f: impl FnMut(Mut<'_, E1>) -> R,
    ) -> R;

    fn with_downcast_b<R>(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        f: impl FnMut(Mut<'_, E2>) -> R,
    ) -> R;

    fn upcast_one_of_two_element(elem: OneOf2<E1, E2>) -> Self::OneOfTwoElement;
    fn update_one_of_two_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfTwoElement>,
        new_elem: OneOf2<E1, E2>,
    );
}

/// The state used to implement `OneOf2<ViewStateA, ViewStateB>`
#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct OneOf2State<InnerStateA, InnerStateB> {
    /// The current state of the inner view.
    inner_state: OneOf2<InnerStateA, InnerStateB>,
    /// The generation this OneOf2 is at.
    ///
    /// If the inner view was A, then B or vice versa,
    /// `View::build` is called again on the new view,
    /// the generation is incremented and used as ViewId in the id_path,
    /// to avoid (possibly async) messages reaching the wrong view,
    /// See the implentations of `ViewSequence` for more details
    generation: u64,
}

impl<V1, V2, Context, T, A> View<T, A, Context> for OneOf2<V1, V2>
where
    T: 'static,
    A: 'static,
    Context: ViewPathTracker + OneOf2Ctx<V1::Element, V2::Element>,
    V1: View<T, A, Context>,
    V2: View<T, A, Context>,
{
    type Element = Context::OneOfTwoElement;

    type ViewState = OneOf2State<V1::ViewState, V2::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, inner_state) = ctx.with_id(ViewId::new(generation), |ctx| match self {
            OneOf2::A(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_two_element(OneOf2::A(element)),
                    OneOf2::A(state),
                )
            }
            OneOf2::B(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_two_element(OneOf2::B(element)),
                    OneOf2::B(state),
                )
            }
        });
        (
            element,
            OneOf2State {
                inner_state,
                generation,
            },
        )
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        match (prev, self) {
            (OneOf2::A(prev), OneOf2::A(new)) => {
                let OneOf2::A(inner_state) = &mut view_state.inner_state else {
                    unreachable!()
                };
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_a(&mut element, |elem| {
                        new.rebuild(prev, inner_state, ctx, elem);
                    });
                });
            }
            (OneOf2::B(prev), OneOf2::B(new)) => {
                let OneOf2::B(inner_state) = &mut view_state.inner_state else {
                    unreachable!()
                };
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_b(&mut element, |elem| {
                        new.rebuild(prev, inner_state, ctx, elem);
                    });
                });
            }
            (OneOf2::B(prev), OneOf2::A(new)) => {
                let OneOf2::B(old_state) = &mut view_state.inner_state else {
                    unreachable!()
                };
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_b(&mut element, |elem| {
                        prev.teardown(old_state, ctx, elem);
                    });
                });
                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                view_state.generation = view_state.generation.wrapping_add(1);

                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    let (new_element, state) = new.build(ctx);
                    view_state.inner_state = OneOf2::A(state);
                    Context::update_one_of_two_element_mut(&mut element, OneOf2::A(new_element));
                });
            }
            (OneOf2::A(prev), OneOf2::B(new)) => {
                let OneOf2::A(old_state) = &mut view_state.inner_state else {
                    unreachable!()
                };
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_a(&mut element, |elem| {
                        prev.teardown(old_state, ctx, elem);
                    });
                });

                view_state.generation = view_state.generation.wrapping_add(1);

                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    let (new_element, state) = new.build(ctx);
                    view_state.inner_state = OneOf2::B(state);
                    Context::update_one_of_two_element_mut(&mut element, OneOf2::B(new_element));
                });
            }
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
            match (self, &mut view_state.inner_state) {
                (OneOf2::A(view), OneOf2::A(state)) => {
                    Context::with_downcast_a(&mut element, |elem| {
                        view.teardown(state, ctx, elem);
                    });
                }
                (OneOf2::B(view), OneOf2::B(state)) => {
                    Context::with_downcast_b(&mut element, |elem| {
                        view.teardown(state, ctx, elem);
                    });
                }
                _ => unreachable!(),
            }
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for OneOf2");
        if start.routing_id() != view_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        match (self, &mut view_state.inner_state) {
            (OneOf2::A(view), OneOf2::A(state)) => view.message(state, rest, message, app_state),
            (OneOf2::B(view), OneOf2::B(state)) => view.message(state, rest, message, app_state),
            _ => unreachable!(),
        }
    }
}
