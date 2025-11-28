// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppendVec, Arg, Count, ElementSplice, MessageCtx, MessageResult, ViewArgument, ViewElement,
    ViewId, ViewPathTracker, ViewSequence,
};

// --- MARK: for ()

impl<State, Action, Context, Element> ViewSequence<State, Action, Context, Element> for ()
where
    State: ViewArgument,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = ();

    const ELEMENTS_COUNT: Count = Count::Zero;

    fn seq_build(
        &self,
        _: &mut Context,
        _: &mut AppendVec<Element>,
        _: Arg<'_, State>,
    ) -> Self::SeqState {
    }

    fn seq_rebuild(
        &self,
        _: &Self,
        _: &mut Self::SeqState,
        _: &mut Context,
        _: &mut impl ElementSplice<Element>,
        _: Arg<'_, State>,
    ) {
    }

    fn seq_teardown(
        &self,
        _seq_state: &mut Self::SeqState,
        _ctx: &mut Context,
        _elements: &mut impl ElementSplice<Element>,
    ) {
    }

    fn seq_message(
        &self,
        _: &mut Self::SeqState,
        message: &mut MessageCtx,
        _: &mut impl ElementSplice<Element>,
        _: Arg<'_, State>,
    ) -> MessageResult<Action> {
        unreachable!("Messages should never be dispatched to an empty tuple {message:?}.");
    }
}

// --- MARK: for (Seq,)

impl<State, Action, Context, Element, Seq> ViewSequence<State, Action, Context, Element> for (Seq,)
where
    State: ViewArgument,
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = Seq::SeqState;

    const ELEMENTS_COUNT: Count = Seq::ELEMENTS_COUNT;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: Arg<'_, State>,
    ) -> Self::SeqState {
        self.0.seq_build(ctx, elements, app_state)
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) {
        self.0
            .seq_rebuild(&prev.0, seq_state, ctx, elements, app_state);
    }

    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        self.0.seq_teardown(seq_state, ctx, elements);
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageCtx,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        self.0.seq_message(seq_state, message, elements, app_state)
    }
}

// --- MARK: for (Seq, ...)

macro_rules! impl_view_tuple {
    (
        // We could use the ${index} metavariable here once it's stable
        // https://veykril.github.io/tlborm/decl-macros/minutiae/metavar-expr.html
        $($marker: ident, $seq: ident, $idx: tt);+
    ) => {
        impl<
                State: ViewArgument,
                Action,
                Context: ViewPathTracker,
                Element: ViewElement,
                $($seq: ViewSequence<State, Action, Context, Element>,)+
            > ViewSequence<State, Action, Context, Element> for ($($seq,)+)

        {
            /// The fields of the inner tuples are (number of widgets to skip, child state).
            type SeqState = ($((usize, $seq::SeqState),)+);

            const ELEMENTS_COUNT: Count = Count::combine([$($seq::ELEMENTS_COUNT,)+]);

            fn seq_build(
                &self,
                ctx: &mut Context,
                elements: &mut AppendVec<Element>,
                mut app_state: Arg<'_, State>,
            ) -> Self::SeqState {
                let start_idx = elements.index();
                ($(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        let this_skip = elements.index() - start_idx;
                        let state = self.$idx.seq_build(ctx, elements, State::reborrow_mut(&mut app_state));
                        (this_skip, state)
                    }),
                )+)
            }

            fn seq_rebuild(
                &self,
                prev: &Self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
                mut app_state: Arg<'_, State>,
            ) {
                let start_idx = elements.index();
                $(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        seq_state.$idx.0 = elements.index() - start_idx;
                        self.$idx.seq_rebuild(&prev.$idx, &mut seq_state.$idx.1, ctx, elements, State::reborrow_mut(&mut app_state));
                    });
                )+
            }

            fn seq_teardown(
                &self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
            ) {
                $(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        self.$idx.seq_teardown(&mut seq_state.$idx.1, ctx, elements)
                    });
                )+
            }

            fn seq_message(
                &self,
                seq_state: &mut Self::SeqState,
                message: &mut MessageCtx,
                elements: &mut impl ElementSplice<Element>,
                mut app_state: Arg<'_, State>,
            ) -> MessageResult<Action> {
                let start = message
                    .take_first()
                    .expect("Id path has elements for tuple");
                match start.routing_id() {
                    $(
                        $idx => {
                            elements.skip(seq_state.$idx.0);
                            self.$idx.seq_message(&mut seq_state.$idx.1, message, elements, State::reborrow_mut(&mut app_state))
                        },
                    )+
                    // If we have received a message, our parent is (mostly) certain that we requested it
                    // The only time that wouldn't be the case is when a generational index has overflowed?
                    _ => unreachable!("Unexpected id path {start:?} in tuple (wants to be routed via {message:?})"),
                }
            }
        }
    };
}

// We implement for tuples of length up to 15. 0 and 1 are special cased to be more efficient
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13; M14, Seq14, 14);
impl_view_tuple!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13; M14, Seq14, 14; M15, Seq15, 15);
