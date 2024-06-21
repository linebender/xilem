// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId, ViewPathTracker};
use paste::paste;

/// This trait allows, specifying a type as `ViewElement`, which should never be constructed or used,
/// but allows downstream implementations to adjust the behaviour of [`PhantomElementCtx::PhantomElement`],
/// e.g. adding trait impls, or a wrapper type, to support features that would depend on the `ViewElement` implementing certain traits, or being a specific type. It's necessary to please the type-checker
///
/// [`PhantomElementCtx::PhantomElement`] is used e.g. in `OneOfCtx` for default elements.
pub trait PhantomElementCtx {
    /// This element is never actually used, it's there to satisfy the type-checker
    type PhantomElement: ViewElement;
}

/// This is used for updating the underlying view element for all the `OneOfN` views.
/// It's mostly there, to avoid hardcoding all of the `OneOfN` variants in traits.
/// The `OneOfN` views are a workaround over using this as `View`,
/// since with defaults for generic parameters rustc is unfortunately not able to infer the default, when the variants are omitted
#[allow(missing_docs)]
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

/// To be able to use `OneOfN` as a [`View`], it's necessary to implement [`OneOfCtx`] for your `ViewCtx` type
pub trait OneOfCtx<
    A: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    B: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    C: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    D: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    E: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    F: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    G: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    H: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
    I: ViewElement = <Self as PhantomElementCtx>::PhantomElement,
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
    fn upcast_one_of_element(elem: OneOf<A, B, C, D, E, F, G, H, I>) -> Self::OneOfElement;

    /// When the variant of the inner view element has changed, the wrapping element needs to be updated, this is used in `View::rebuild`
    fn update_one_of_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
        new_elem: OneOf<A, B, C, D, E, F, G, H, I>,
    );
}

#[doc(hidden)] // Implementation detail, `OneOfState` is public because of trait visibility rules
mod hidden {
    /// The state used to implement `View` for `OneOfN`
    #[allow(unreachable_pub)]
    pub struct OneOfState<A = (), B = (), C = (), D = (), E = (), F = (), G = (), H = (), I = ()> {
        /// The current state of the inner view or view sequence.
        pub(super) inner_state: super::OneOf<A, B, C, D, E, F, G, H, I>,
        /// The generation this OneOfN is at.
        ///
        /// If the variant of `OneOfN` has changed, i.e. the type of the inner view,
        /// the generation is incremented and used as ViewId in the id_path,
        /// to avoid (possibly async) messages reaching the wrong view,
        /// See the implementations of other `ViewSequence`s for more details
        pub(super) generation: u64,
    }
}

macro_rules! one_of {
    ($ty_name: ident, $num_word: ident, $($variant: ident),+) => {
        // This is not optimal, as it requires $variant to contain at least `A` and `B`, but better than no doc example...
        paste!{
        /// Statically typed alternative to the type-erased [`AnyView`](`crate::AnyView`).
        ///
        #[doc = "This view container can switch between " $num_word " different views."]
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```ignore
        #[doc = "let mut v = " $ty_name "::A(my_view());"]
        #[doc = "v = " $ty_name "::B(my_other_view());"]
        /// ```
        #[allow(missing_docs)]
        pub enum $ty_name<$($variant),+> {
            $($variant($variant)),+
        }

        impl<T, $($variant: AsRef<T>),+> AsRef<T> for $ty_name<$($variant),+>
        {
            fn as_ref(&self) -> &T {
                match self {
                    $($ty_name::$variant(e) => <$variant as AsRef<T>>::as_ref(e)),+
                }
            }
        }

        impl<Context, State, Action, $($variant),+> View<State, Action, Context> for $ty_name<$($variant),+>
        where
            State: 'static,
            Action: 'static,
            Context: ViewPathTracker + OneOfCtx<$($variant::Element),+>,
            $($variant: View<State, Action, Context>),+
        {
            type Element = Context::OneOfElement;

            type ViewState = hidden::OneOfState<$($variant::ViewState),+>;

            fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
                let generation = 0;
                let (element, inner_state) =
                    ctx.with_id(ViewId::new(generation), |ctx| match self {
                        $($ty_name::$variant(e) => {
                            let (element, state) = e.build(ctx);
                            (
                                Context::upcast_one_of_element(OneOf::$variant(element)),
                                OneOf::$variant(state),
                            )
                        }),+
                    });
                (
                    element,
                    hidden::OneOfState {
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
                // Type of the inner `View` stayed the same
                match (prev, self, &mut view_state.inner_state) {
                    $(($ty_name::$variant(prev), $ty_name::$variant(new), OneOf::$variant(inner_state)) => {
                        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                            Context::[<with_downcast_ $variant:lower>](&mut element, |elem| {
                                new.rebuild(prev, inner_state, ctx, elem);
                            });
                        });
                        return element;
                    })+
                    _ => ()
                };

                // View has changed type, teardown the old view
                // we can't use Self::teardown, because we still need access to the element

                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    match (prev, &mut view_state.inner_state) {
                        $(($ty_name::$variant(prev), OneOf::$variant(old_state)) => {
                            Context::[<with_downcast_ $variant:lower>](&mut element, |elem| {
                                prev.teardown(old_state, ctx, elem);
                            });
                        })+
                        _ => unreachable!(),
                    };
                });

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                view_state.generation = view_state.generation.wrapping_add(1);

                // Create the new view

                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    match self {
                        $($ty_name::$variant(new) => {
                            let (new_element, state) = new.build(ctx);
                            view_state.inner_state = OneOf::$variant(state);
                            Context::update_one_of_element_mut(
                                &mut element,
                                OneOf::$variant(new_element),
                            );
                        })+
                    };
                });
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
                        $(($ty_name::$variant(view), OneOf::$variant(state)) => {
                            Context::[<with_downcast_ $variant:lower>](&mut element, |elem| {
                                view.teardown(state, ctx, elem);
                            });
                        })+
                        _ => unreachable!(),
                    }
                });
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                let (start, rest) = id_path
                    .split_first()
                    .expect(concat!("Id path has elements for ", stringify!($ty_name)));
                if start.routing_id() != view_state.generation {
                    // The message was sent to a previous edition of the inner value
                    return MessageResult::Stale(message);
                }
                match (self, &mut view_state.inner_state) {
                    $(($ty_name::$variant(view), OneOf::$variant(state)) => {
                        view.message(state, rest, message, app_state)
                    }),+
                    _ => unreachable!(),
                }
            }
        }
        }
    };
}

one_of!(OneOf2, two, A, B);
one_of!(OneOf3, three, A, B, C);
one_of!(OneOf4, four, A, B, C, D);
one_of!(OneOf5, five, A, B, C, D, E);
one_of!(OneOf6, six, A, B, C, D, E, F);
one_of!(OneOf7, seven, A, B, C, D, E, F, G);
one_of!(OneOf8, eight, A, B, C, D, E, F, G, H);
one_of!(OneOf9, nine, A, B, C, D, E, F, G, H, I);
