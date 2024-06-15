// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, View, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};
use paste::paste;
// Used in doc comment
#[allow(unused_imports)]
use crate::AnyView;

macro_rules! one_of {
    ($ty_name: ident, $num_word: ident, $($variant: ident),+) => {
        // This is not optimal, as it requires $variant to contain at least `A` and `B`, but better than no doc example...
        paste!{
        /// Statically typed alternative to the type-erased [`AnyView`].
        ///
        #[doc = "This view container can switch between " $num_word " different views."]
        ///
        /// It can also be used for alternating between different [`ViewSequence`]s
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```ignore
        /// // As view
        #[doc = "let mut v = " $ty_name "::A(my_view());"]
        #[doc = "v = " $ty_name "::B(my_other_view());"]
        /// // As view sequence
        #[doc = "let mut seq = " $ty_name "::A((my_view(), my_other_view()));"]
        #[doc = "seq = " $ty_name "::B(vec![my_view()]);"]
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

        #[doc = "To be able to use [`" $ty_name "`] as a [`View`], it's necessary to implement [`" $ty_name Ctx "`] for your `ViewCtx` type."]
        pub trait [<$ty_name Ctx>]<$($variant: ViewElement),+> {
            /// Element wrapper, that holds the current view element variant
            type [<OneOf $num_word:camel Element>]: ViewElement;

            $(
            #[doc = "Casts the view element `elem` to the [`" $ty_name "::" $variant "`] variant."]
            /// `f` needs to be invoked with that inner [`ViewElement`]
            fn [<with_downcast_ $variant:lower>](
                elem: &mut Mut<'_, Self::[<OneOf $num_word:camel Element>]>,
                f: impl FnOnce(Mut<'_, $variant>),
            );
            )+

            /// Creates the wrapping element, this is used in [`View::build`] to wrap the inner view element variant
            fn [<upcast_one_of_ $num_word _element>](elem: $ty_name<$($variant),+>) -> Self::[<OneOf $num_word:camel Element>];

            /// When the variant of the inner view element has changed, the wrapping element needs to be updated, this is used in [`View::rebuild`]
            fn [<update_one_of_ $num_word _element_mut>](
                elem_mut: &mut Mut<'_, Self::[<OneOf $num_word:camel Element>]>,
                new_elem: $ty_name<$($variant),+>,
            );
        }

        /// The state used to implement `View` or `ViewSequence` for `OneOfN`
        #[doc(hidden)] // Implementation detail, public because of trait visibility rules
        pub struct [<$ty_name State>]<$($variant),+> {
            /// The current state of the inner view or view sequence.
            inner_state: $ty_name<$($variant),+>,
            /// The generation this OneOfN is at.
            ///
            /// If the variant of `OneOfN` has changed, i.e. the type of the inner view,
            /// the generation is incremented and used as ViewId in the id_path,
            /// to avoid (possibly async) messages reaching the wrong view,
            /// See the implementations of other `ViewSequence`s for more details
            generation: u64,
        }

        impl<Context, State, Action, $($variant),+> View<State, Action, Context> for $ty_name<$($variant),+>
        where
            State: 'static,
            Action: 'static,
            Context: ViewPathTracker + [<$ty_name Ctx>]<$($variant::Element),+>,
            $($variant: View<State, Action, Context>),+
        {
            type Element = Context::[<OneOf $num_word:camel Element>];

            type ViewState = [<$ty_name State>]<$($variant::ViewState),+>;

            fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
                let generation = 0;
                let (element, inner_state) =
                    ctx.with_id(ViewId::new(generation), |ctx| match self {
                        $($ty_name::$variant(e) => {
                            let (element, state) = e.build(ctx);
                            (
                                Context::[<upcast_one_of_ $num_word _element>]($ty_name::$variant(element)),
                                $ty_name::$variant(state),
                            )
                        }),+
                    });
                (
                    element,
                    [<$ty_name State>] {
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
                    $(($ty_name::$variant(prev), $ty_name::$variant(new), $ty_name::$variant(inner_state)) => {
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
                        $(($ty_name::$variant(prev), $ty_name::$variant(old_state)) => {
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
                            view_state.inner_state = $ty_name::$variant(state);
                            Context::[<update_one_of_ $num_word _element_mut>](
                                &mut element,
                                $ty_name::$variant(new_element),
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
                        $(($ty_name::$variant(view), $ty_name::$variant(state)) => {
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
                    $(($ty_name::$variant(view), $ty_name::$variant(state)) => {
                        view.message(state, rest, message, app_state)
                    }),+
                    _ => unreachable!(),
                }
            }
        }

        impl<State, Action, Context, Element, $([<Marker $variant>],)+ $($variant),+>
            ViewSequence<State, Action, Context, Element, $ty_name<$([<Marker $variant>]),+>>
            for $ty_name<$($variant),+>
        where
            $($variant: ViewSequence<State, Action, Context, Element, [<Marker $variant>]>,)+
            Context: ViewPathTracker,
            Element: ViewElement,
        {
            type SeqState = [<$ty_name State>]<$($variant::SeqState),+>;

            fn seq_build(
                &self,
                ctx: &mut Context,
                elements: &mut AppendVec<Element>,
            ) -> Self::SeqState {
                let generation = 0;
                let inner_state = ctx.with_id(ViewId::new(generation), |ctx| match self {
                    $($ty_name::$variant(e) => $ty_name::$variant(e.seq_build(ctx, elements))),+
                });
                [<$ty_name State>] {
                    inner_state,
                    generation,
                }
            }

            fn seq_rebuild(
                &self,
                prev: &Self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
            ) {
                // Type of the inner `ViewSequence` stayed the same
                match (prev, self, &mut seq_state.inner_state) {
                    $(($ty_name::$variant(prev), $ty_name::$variant(new), $ty_name::$variant(inner_state)) => {
                        ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                            new.seq_rebuild(prev, inner_state, ctx, elements);
                        });
                        return;
                    })+
                    _ => (),
                };

                // `ViewSequence` has changed type, teardown the old view sequence
                prev.seq_teardown(seq_state, ctx, elements);

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                seq_state.generation = seq_state.generation.wrapping_add(1);

                // Create the new view sequence

                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    match self {
                        $($ty_name::$variant(new) => {
                            seq_state.inner_state = $ty_name::$variant(
                                elements.with_scratch(|elements| new.seq_build(ctx, elements)),
                            );
                        })+
                    };
                });
            }

            fn seq_teardown(
                &self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
            ) {
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    match (self, &mut seq_state.inner_state) {
                        $(($ty_name::$variant(view), $ty_name::$variant(state)) => {
                            view.seq_teardown(state, ctx, elements);
                        })+
                        _ => unreachable!(),
                    }
                });
            }

            fn seq_message(
                &self,
                seq_state: &mut Self::SeqState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                let (start, rest) = id_path
                    .split_first()
                    .expect("Id path has elements for OneOfN");
                if start.routing_id() != seq_state.generation {
                    // The message was sent to a previous edition of the inner value
                    return MessageResult::Stale(message);
                }
                match (self, &mut seq_state.inner_state) {
                    $(($ty_name::$variant(view), $ty_name::$variant(state)) => {
                        view.seq_message(state, rest, message, app_state)
                    })+
                    _ => MessageResult::Stale(message),
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
