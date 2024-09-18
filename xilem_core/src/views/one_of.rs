// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).

use crate::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker, ViewPathTracker};
use hidden::OneOfState;

/// This trait allows, specifying a type as `ViewElement`, which should never be constructed or used,
/// but allows downstream implementations to adjust the behaviour of [`PhantomElementCtx::PhantomElement`],
/// e.g. adding trait impls, or a wrapper type, to support features that would depend on the `ViewElement` implementing certain traits, or being a specific type.
///
/// It's necessary to please the type-checker
///
/// [`PhantomElementCtx::PhantomElement`] is used e.g. in `OneOfCtx` for default elements.
pub trait PhantomElementCtx: ViewPathTracker {
    /// This element is never actually used, it's there to satisfy the type-checker
    type PhantomElement: ViewElement;
}

/// A [`View`] which can be one of nine inner view types.
#[allow(missing_docs)] // On variants
pub enum OneOf<A = (), B = (), C = (), D = (), E = (), F = (), G = (), H = (), I = ()> {
    A(A),
    B(B),
    C(C),
    D(D),
    E(E),
    F(F),
    G(G),
    H(H),
    I(I),
}

/// An alias for the never type,
type N = hidden::Never;
/// A [`View`] which can be either of two inner view types.
///
/// Alias for [`OneOf2`] under a more familiar name.
pub type Either<A, B> = OneOf2<A, B>;
/// A [`View`] which can be either of two inner view types.
pub type OneOf2<A, B> = OneOf<A, B, N, N, N, N, N, N, N>;
/// A [`View`] which can be any one of three inner view types.
pub type OneOf3<A, B, C> = OneOf<A, B, C, N, N, N, N, N, N>;
/// A [`View`] which can be any one of four inner view types.
pub type OneOf4<A, B, C, D> = OneOf<A, B, C, D, N, N, N, N, N>;
/// A [`View`] which can be any one of five inner view types.
pub type OneOf5<A, B, C, D, E> = OneOf<A, B, C, D, E, N, N, N, N>;
/// A [`View`] which can be any one of six inner view types.
pub type OneOf6<A, B, C, D, E, F> = OneOf<A, B, C, D, E, F, N, N, N>;
/// A [`View`] which can be any one of seven inner view types.
pub type OneOf7<A, B, C, D, E, F, G> = OneOf<A, B, C, D, E, F, G, N, N>;
/// A [`View`] which can be any one of eight inner view types.
pub type OneOf8<A, B, C, D, E, F, G, H> = OneOf<A, B, C, D, E, F, G, H, N>;
/// A [`View`] which can be any one of nine inner view types.
pub type OneOf9<A, B, C, D, E, F, G, H, I> = OneOf<A, B, C, D, E, F, G, H, I>;

impl<T, A, B, C, D, E, F, G, H, I> AsRef<T> for OneOf<A, B, C, D, E, F, G, H, I>
where
    A: AsRef<T>,
    B: AsRef<T>,
    C: AsRef<T>,
    D: AsRef<T>,
    E: AsRef<T>,
    F: AsRef<T>,
    G: AsRef<T>,
    H: AsRef<T>,
    I: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        match self {
            OneOf::A(e) => <A as AsRef<T>>::as_ref(e),
            OneOf::B(e) => <B as AsRef<T>>::as_ref(e),
            OneOf::C(e) => <C as AsRef<T>>::as_ref(e),
            OneOf::D(e) => <D as AsRef<T>>::as_ref(e),
            OneOf::E(e) => <E as AsRef<T>>::as_ref(e),
            OneOf::F(e) => <F as AsRef<T>>::as_ref(e),
            OneOf::G(e) => <G as AsRef<T>>::as_ref(e),
            OneOf::H(e) => <H as AsRef<T>>::as_ref(e),
            OneOf::I(e) => <I as AsRef<T>>::as_ref(e),
        }
    }
}

/// A context type which can support [`OneOf9`] and [related views](super::one_of).
///
/// This should be implemented by users of Xilem Core.
pub trait OneOfCtx<
    A: ViewElement,
    B: ViewElement,
    C: ViewElement,
    D: ViewElement,
    E: ViewElement,
    F: ViewElement,
    G: ViewElement,
    H: ViewElement,
    I: ViewElement,
>: PhantomElementCtx
{
    /// Element wrapper, that holds the current view element variant
    type OneOfElement: ViewElement;

    /// Casts the view element `elem` to the `OneOf::A` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_a(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, A>));

    /// Casts the view element `elem` to the `OneOf::B` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_b(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, B>));

    /// Casts the view element `elem` to the `OneOf::C` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_c(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, C>));

    /// Casts the view element `elem` to the `OneOf::D` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_d(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, D>));

    /// Casts the view element `elem` to the `OneOf::E` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_e(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, E>));

    /// Casts the view element `elem` to the `OneOf::F` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_f(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, F>));

    /// Casts the view element `elem` to the `OneOf::G` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_g(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, G>));

    /// Casts the view element `elem` to the `OneOf::H` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_h(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, H>));

    /// Casts the view element `elem` to the `OneOf::I` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_i(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, I>));

    /// Creates the wrapping element, this is used in `View::build` to wrap the inner view element variant
    fn upcast_one_of_element(
        &mut self,
        elem: OneOf<A, B, C, D, E, F, G, H, I>,
    ) -> Self::OneOfElement;

    /// When the variant of the inner view element has changed, the wrapping element needs to be updated, this is used in `View::rebuild`
    fn update_one_of_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
        new_elem: OneOf<A, B, C, D, E, F, G, H, I>,
    );
}

impl<A, B, C, D, E, F, G, H, I> ViewMarker for OneOf<A, B, C, D, E, F, G, H, I> {}
/// The `OneOf` types and `Either` are [`View`]s if all of their possible types are themselves `View`s.
impl<State, Action, Context, Message, A, B, C, D, E, F, G, H, I>
    View<State, Action, Context, Message> for OneOf<A, B, C, D, E, F, G, H, I>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker
        + OneOfCtx<
            A::Element,
            B::Element,
            C::Element,
            D::Element,
            E::Element,
            F::Element,
            G::Element,
            H::Element,
            I::Element,
        >,
    A: View<State, Action, Context, Message>,
    B: View<State, Action, Context, Message>,
    C: View<State, Action, Context, Message>,
    D: View<State, Action, Context, Message>,
    E: View<State, Action, Context, Message>,
    F: View<State, Action, Context, Message>,
    G: View<State, Action, Context, Message>,
    H: View<State, Action, Context, Message>,
    I: View<State, Action, Context, Message>,
{
    #[doc(hidden)]
    type Element = Context::OneOfElement;

    #[doc(hidden)]
    type ViewState = hidden::OneOfState<
        A::ViewState,
        B::ViewState,
        C::ViewState,
        D::ViewState,
        E::ViewState,
        F::ViewState,
        G::ViewState,
        H::ViewState,
        I::ViewState,
    >;

    #[doc(hidden)]
    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, state) = ctx.with_id(ViewId::new(generation), |ctx| match self {
            OneOf::A(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::A(element), OneOf::A(state))
            }
            OneOf::B(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::B(element), OneOf::B(state))
            }
            OneOf::C(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::C(element), OneOf::C(state))
            }
            OneOf::D(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::D(element), OneOf::D(state))
            }
            OneOf::E(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::E(element), OneOf::E(state))
            }
            OneOf::F(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::F(element), OneOf::F(state))
            }
            OneOf::G(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::G(element), OneOf::G(state))
            }
            OneOf::H(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::H(element), OneOf::H(state))
            }
            OneOf::I(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::I(element), OneOf::I(state))
            }
        });
        (
            ctx.upcast_one_of_element(element),
            OneOfState {
                generation,
                inner_state: state,
            },
        )
    }

    #[doc(hidden)]
    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let id = ViewId::new(view_state.generation);
        // If both elements are of the same type, do a simple rebuild
        match (self, prev, &mut view_state.inner_state) {
            (OneOf::A(this), OneOf::A(prev), OneOf::A(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_a(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::B(this), OneOf::B(prev), OneOf::B(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_b(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::C(this), OneOf::C(prev), OneOf::C(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_c(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::D(this), OneOf::D(prev), OneOf::D(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_d(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::E(this), OneOf::E(prev), OneOf::E(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_e(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::F(this), OneOf::F(prev), OneOf::F(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_f(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::G(this), OneOf::G(prev), OneOf::G(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_g(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::H(this), OneOf::H(prev), OneOf::H(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_h(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            (OneOf::I(this), OneOf::I(prev), OneOf::I(ref mut state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_i(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element);
                    });
                });
                return element;
            }
            _ => (),
        }

        // We're changing the type of the view. Teardown the old version
        ctx.with_id(id, |ctx| match (prev, &mut view_state.inner_state) {
            (OneOf::A(prev), OneOf::A(ref mut state)) => {
                Context::with_downcast_a(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::B(prev), OneOf::B(ref mut state)) => {
                Context::with_downcast_b(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::C(prev), OneOf::C(ref mut state)) => {
                Context::with_downcast_c(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::D(prev), OneOf::D(ref mut state)) => {
                Context::with_downcast_d(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::E(prev), OneOf::E(ref mut state)) => {
                Context::with_downcast_e(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::F(prev), OneOf::F(ref mut state)) => {
                Context::with_downcast_f(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::G(prev), OneOf::G(ref mut state)) => {
                Context::with_downcast_g(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::H(prev), OneOf::H(ref mut state)) => {
                Context::with_downcast_h(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (OneOf::I(prev), OneOf::I(ref mut state)) => {
                Context::with_downcast_i(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            _ => unreachable!(),
        });

        // Overflow handling: u64 can never realistically overflow
        view_state.generation = view_state.generation.wrapping_add(1);

        // And rebuild the new one
        let id = ViewId::new(view_state.generation);
        let (new_element, state) = ctx.with_id(id, |ctx| match self {
            OneOf::A(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::A(element), OneOf::A(state))
            }
            OneOf::B(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::B(element), OneOf::B(state))
            }
            OneOf::C(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::C(element), OneOf::C(state))
            }
            OneOf::D(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::D(element), OneOf::D(state))
            }
            OneOf::E(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::E(element), OneOf::E(state))
            }
            OneOf::F(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::F(element), OneOf::F(state))
            }
            OneOf::G(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::G(element), OneOf::G(state))
            }
            OneOf::H(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::H(element), OneOf::H(state))
            }
            OneOf::I(v) => {
                let (element, state) = v.build(ctx);
                (OneOf::I(element), OneOf::I(state))
            }
        });
        view_state.inner_state = state;
        Context::update_one_of_element_mut(&mut element, new_element);

        element
    }

    #[doc(hidden)]
    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
            match (self, &mut view_state.inner_state) {
                (OneOf::A(v), OneOf::A(ref mut state)) => {
                    Context::with_downcast_a(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::B(v), OneOf::B(ref mut state)) => {
                    Context::with_downcast_b(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::C(v), OneOf::C(ref mut state)) => {
                    Context::with_downcast_c(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::D(v), OneOf::D(ref mut state)) => {
                    Context::with_downcast_d(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::E(v), OneOf::E(ref mut state)) => {
                    Context::with_downcast_e(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::F(v), OneOf::F(ref mut state)) => {
                    Context::with_downcast_f(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::G(v), OneOf::G(ref mut state)) => {
                    Context::with_downcast_g(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::H(v), OneOf::H(ref mut state)) => {
                    Context::with_downcast_h(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (OneOf::I(v), OneOf::I(ref mut state)) => {
                    Context::with_downcast_i(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                _ => unreachable!(),
            }
        });
    }

    #[doc(hidden)]
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for OneOf");
        if start.routing_id() != view_state.generation {
            return MessageResult::Stale(message);
        }
        match (self, &mut view_state.inner_state) {
            (OneOf::A(v), OneOf::A(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::B(v), OneOf::B(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::C(v), OneOf::C(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::D(v), OneOf::D(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::E(v), OneOf::E(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::F(v), OneOf::F(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::G(v), OneOf::G(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::H(v), OneOf::H(ref mut state)) => v.message(state, rest, message, app_state),
            (OneOf::I(v), OneOf::I(ref mut state)) => v.message(state, rest, message, app_state),
            _ => unreachable!(),
        }
    }
}

// Because `OneOfState` is not public API, but is part of a public trait `impl`, it must be marked pub, but we don't want
// to export it. Since this (`one_of`) module is public, we create a new module, allowing it to be pub but not exposed.
#[doc(hidden)]
mod hidden {
    use crate::{View, ViewMarker};

    use super::PhantomElementCtx;

    #[allow(unreachable_pub)]
    pub enum Never {}

    impl ViewMarker for Never {}
    impl<State, Action, Context: PhantomElementCtx, Message> View<State, Action, Context, Message>
        for Never
    {
        type Element = Context::PhantomElement;

        type ViewState = Never;

        fn build(&self, _: &mut Context) -> (Self::Element, Self::ViewState) {
            match *self {}
        }

        fn rebuild<'el>(
            &self,
            _: &Self,
            _: &mut Self::ViewState,
            _: &mut Context,
            _: crate::Mut<'el, Self::Element>,
        ) -> crate::Mut<'el, Self::Element> {
            match *self {}
        }

        fn teardown(
            &self,
            _: &mut Self::ViewState,
            _: &mut Context,
            _: crate::Mut<'_, Self::Element>,
        ) {
            match *self {}
        }

        fn message(
            &self,
            _: &mut Self::ViewState,
            _: &[crate::ViewId],
            _: Message,
            _: &mut State,
        ) -> crate::MessageResult<Action, Message> {
            match *self {}
        }
    }
    /// The state used to implement `View` for `OneOfN`
    #[allow(unreachable_pub)]
    pub struct OneOfState<A, B, C, D, E, F, G, H, I> {
        /// The current state of the inner view or view sequence.
        pub(super) inner_state: super::OneOf<A, B, C, D, E, F, G, H, I>,
        /// The generation this `OneOfN` is at.
        ///
        /// If the variant of `OneOfN` has changed, i.e. the type of the inner view,
        /// the generation is incremented and used as `ViewId` in the `id_path`,
        /// to avoid (possibly async) messages reaching the wrong view,
        /// See the implementations of other `ViewSequence`s for more details
        pub(super) generation: u64,
    }
}
