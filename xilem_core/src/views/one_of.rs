// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).

use hidden::OneOfState;

use crate::{
    Arg, MessageContext, MessageResult, Mut, View, ViewArgument, ViewElement, ViewId, ViewMarker,
    ViewPathTracker,
};

/// This trait allows, specifying a type as `ViewElement`, which should never be constructed or used.
///
/// But it allows downstream implementations to adjust the behaviour of [`PhantomElementCtx::PhantomElement`],
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
#[derive(Debug)]
#[must_use = "View values do nothing unless provided to Xilem."]
#[expect(missing_docs, reason = "No need to document all variants")]
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
            Self::A(e) => <A as AsRef<T>>::as_ref(e),
            Self::B(e) => <B as AsRef<T>>::as_ref(e),
            Self::C(e) => <C as AsRef<T>>::as_ref(e),
            Self::D(e) => <D as AsRef<T>>::as_ref(e),
            Self::E(e) => <E as AsRef<T>>::as_ref(e),
            Self::F(e) => <F as AsRef<T>>::as_ref(e),
            Self::G(e) => <G as AsRef<T>>::as_ref(e),
            Self::H(e) => <H as AsRef<T>>::as_ref(e),
            Self::I(e) => <I as AsRef<T>>::as_ref(e),
        }
    }
}

impl<T, A, B, C, D, E, F, G, H, I> AsMut<T> for OneOf<A, B, C, D, E, F, G, H, I>
where
    A: AsMut<T>,
    B: AsMut<T>,
    C: AsMut<T>,
    D: AsMut<T>,
    E: AsMut<T>,
    F: AsMut<T>,
    G: AsMut<T>,
    H: AsMut<T>,
    I: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        match self {
            Self::A(e) => <A as AsMut<T>>::as_mut(e),
            Self::B(e) => <B as AsMut<T>>::as_mut(e),
            Self::C(e) => <C as AsMut<T>>::as_mut(e),
            Self::D(e) => <D as AsMut<T>>::as_mut(e),
            Self::E(e) => <E as AsMut<T>>::as_mut(e),
            Self::F(e) => <F as AsMut<T>>::as_mut(e),
            Self::G(e) => <G as AsMut<T>>::as_mut(e),
            Self::H(e) => <H as AsMut<T>>::as_mut(e),
            Self::I(e) => <I as AsMut<T>>::as_mut(e),
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
    fn with_downcast_a<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, A>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::B` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_b<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, B>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::C` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_c<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, C>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::D` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_d<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, D>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::E` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_e<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, E>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::F` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_f<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, F>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::G` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_g<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, G>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::H` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_h<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, H>) -> R,
    ) -> R;

    /// Casts the view element `elem` to the `OneOf::I` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_i<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, I>) -> R,
    ) -> R;

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
impl<State, Action, Context, A, B, C, D, E, F, G, H, I> View<State, Action, Context>
    for OneOf<A, B, C, D, E, F, G, H, I>
where
    State: ViewArgument,
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
    A: View<State, Action, Context>,
    B: View<State, Action, Context>,
    C: View<State, Action, Context>,
    D: View<State, Action, Context>,
    E: View<State, Action, Context>,
    F: View<State, Action, Context>,
    G: View<State, Action, Context>,
    H: View<State, Action, Context>,
    I: View<State, Action, Context>,
{
    #[doc(hidden)]
    type Element = Context::OneOfElement;

    #[doc(hidden)]
    type ViewState = OneOfState<
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
    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, state) = ctx.with_id(ViewId::new(generation), |ctx| match self {
            Self::A(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::A(new_element), OneOf::A(state))
            }
            Self::B(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::B(new_element), OneOf::B(state))
            }
            Self::C(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::C(new_element), OneOf::C(state))
            }
            Self::D(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::D(new_element), OneOf::D(state))
            }
            Self::E(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::E(new_element), OneOf::E(state))
            }
            Self::F(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::F(new_element), OneOf::F(state))
            }
            Self::G(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::G(new_element), OneOf::G(state))
            }
            Self::H(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::H(new_element), OneOf::H(state))
            }
            Self::I(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::I(new_element), OneOf::I(state))
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
    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        let id = ViewId::new(view_state.generation);
        // If both elements are of the same type, do a simple rebuild
        match (self, prev, &mut view_state.inner_state) {
            (Self::A(this), Self::A(prev), OneOf::A(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_a(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::B(this), Self::B(prev), OneOf::B(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_b(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::C(this), Self::C(prev), OneOf::C(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_c(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::D(this), Self::D(prev), OneOf::D(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_d(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::E(this), Self::E(prev), OneOf::E(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_e(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::F(this), Self::F(prev), OneOf::F(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_f(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::G(this), Self::G(prev), OneOf::G(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_g(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::H(this), Self::H(prev), OneOf::H(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_h(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            (Self::I(this), Self::I(prev), OneOf::I(state)) => {
                ctx.with_id(id, |ctx| {
                    Context::with_downcast_i(&mut element, |element| {
                        this.rebuild(prev, state, ctx, element, app_state);
                    });
                });
                return;
            }
            _ => (),
        }

        // We're changing the type of the view. Teardown the old version
        ctx.with_id(id, |ctx| match (prev, &mut view_state.inner_state) {
            (Self::A(prev), OneOf::A(state)) => {
                Context::with_downcast_a(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::B(prev), OneOf::B(state)) => {
                Context::with_downcast_b(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::C(prev), OneOf::C(state)) => {
                Context::with_downcast_c(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::D(prev), OneOf::D(state)) => {
                Context::with_downcast_d(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::E(prev), OneOf::E(state)) => {
                Context::with_downcast_e(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::F(prev), OneOf::F(state)) => {
                Context::with_downcast_f(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::G(prev), OneOf::G(state)) => {
                Context::with_downcast_g(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::H(prev), OneOf::H(state)) => {
                Context::with_downcast_h(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            (Self::I(prev), OneOf::I(state)) => {
                Context::with_downcast_i(&mut element, |element| {
                    prev.teardown(state, ctx, element);
                });
            }
            _ => unreachable!(),
        });

        // Overflow handling: u64 can never realistically overflow
        view_state.generation = view_state.generation.wrapping_add(1);

        // And rebuild the new one
        #[expect(clippy::shadow_unrelated, reason = "The old value is no longer valid")]
        let id = ViewId::new(view_state.generation);
        let (new_element, state) = ctx.with_id(id, |ctx| match self {
            Self::A(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::A(new_element), OneOf::A(state))
            }
            Self::B(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::B(new_element), OneOf::B(state))
            }
            Self::C(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::C(new_element), OneOf::C(state))
            }
            Self::D(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::D(new_element), OneOf::D(state))
            }
            Self::E(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::E(new_element), OneOf::E(state))
            }
            Self::F(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::F(new_element), OneOf::F(state))
            }
            Self::G(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::G(new_element), OneOf::G(state))
            }
            Self::H(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::H(new_element), OneOf::H(state))
            }
            Self::I(v) => {
                let (new_element, state) = v.build(ctx, app_state);
                (OneOf::I(new_element), OneOf::I(state))
            }
        });
        view_state.inner_state = state;
        Context::update_one_of_element_mut(&mut element, new_element);
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
                (Self::A(v), OneOf::A(state)) => {
                    Context::with_downcast_a(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::B(v), OneOf::B(state)) => {
                    Context::with_downcast_b(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::C(v), OneOf::C(state)) => {
                    Context::with_downcast_c(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::D(v), OneOf::D(state)) => {
                    Context::with_downcast_d(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::E(v), OneOf::E(state)) => {
                    Context::with_downcast_e(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::F(v), OneOf::F(state)) => {
                    Context::with_downcast_f(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::G(v), OneOf::G(state)) => {
                    Context::with_downcast_g(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::H(v), OneOf::H(state)) => {
                    Context::with_downcast_h(&mut element, |element| {
                        v.teardown(state, ctx, element);
                    });
                }
                (Self::I(v), OneOf::I(state)) => {
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
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for OneOf");
        if start.routing_id() != view_state.generation {
            return MessageResult::Stale;
        }
        match (self, &mut view_state.inner_state) {
            (Self::A(v), OneOf::A(state)) => Context::with_downcast_a(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::B(v), OneOf::B(state)) => Context::with_downcast_b(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::C(v), OneOf::C(state)) => Context::with_downcast_c(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::D(v), OneOf::D(state)) => Context::with_downcast_d(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::E(v), OneOf::E(state)) => Context::with_downcast_e(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::F(v), OneOf::F(state)) => Context::with_downcast_f(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::G(v), OneOf::G(state)) => Context::with_downcast_g(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::H(v), OneOf::H(state)) => Context::with_downcast_h(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            (Self::I(v), OneOf::I(state)) => Context::with_downcast_i(&mut element, |element| {
                v.message(state, message, element, app_state)
            }),
            _ => unreachable!(),
        }
    }
}

// Because `OneOfState` is not public API, but is part of a public trait `impl`, it must be marked pub, but we don't want
// to export it. Since this (`one_of`) module is public, we create a new module, allowing it to be pub but not exposed.
#[doc(hidden)]
mod hidden {
    use super::PhantomElementCtx;
    use crate::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};

    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Debug)]
    pub enum Never {}

    impl ViewMarker for Never {}
    impl<State, Action, Context: PhantomElementCtx> View<State, Action, Context> for Never
    where
        State: ViewArgument,
    {
        type Element = Context::PhantomElement;

        type ViewState = Self;

        fn build(&self, _: &mut Context, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
            match *self {}
        }

        fn rebuild(
            &self,
            _: &Self,
            _: &mut Self::ViewState,
            _: &mut Context,
            _: Mut<'_, Self::Element>,
            _: Arg<'_, State>,
        ) {
            match *self {}
        }

        fn teardown(&self, _: &mut Self::ViewState, _: &mut Context, _: Mut<'_, Self::Element>) {
            match *self {}
        }

        fn message(
            &self,
            _: &mut Self::ViewState,
            _: &mut MessageContext,
            _: Mut<'_, Self::Element>,
            _: Arg<'_, State>,
        ) -> crate::MessageResult<Action> {
            match *self {}
        }
    }
    /// The state used to implement `View` for `OneOfN`
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Debug)]
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
