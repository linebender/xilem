// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppendVec, Count, ElementSplice, MessageCtx, MessageResult, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};

impl<State, Action, Context, Element, Seq, const N: usize>
    ViewSequence<State, Action, Context, Element> for [Seq; N]
where
    State: 'static,
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    /// The fields of the tuple are (number of widgets to skip, child `SeqState`).
    type SeqState = [(usize, Seq::SeqState); N];

    #[doc(hidden)]
    // TODO: Optimise?
    const ELEMENTS_COUNT: Count = Count::combine([Seq::ELEMENTS_COUNT; N]);

    #[doc(hidden)]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
    ) -> Self::SeqState {
        let start_idx = elements.index();
        // there's no enumerate directly on an array
        let mut idx = 0;
        self.each_ref().map(|vs| {
            let this_skip = elements.index() - start_idx;
            let state = ctx.with_id(ViewId::new(idx), |ctx| {
                vs.seq_build(ctx, elements, app_state)
            });
            idx += 1;
            (this_skip, state)
        })
    }

    #[doc(hidden)]
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        let start_idx = elements.index();
        for (idx, ((seq, prev_seq), (this_skip, state))) in
            self.iter().zip(prev).zip(seq_state).enumerate()
        {
            *this_skip = elements.index() - start_idx;
            ctx.with_id(
                ViewId::new(idx.try_into().expect(
                    "ViewSequence arrays with more than u64::MAX + 1 elements not supported",
                )),
                |ctx| {
                    seq.seq_rebuild(prev_seq, state, ctx, elements, app_state);
                },
            );
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageCtx,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for [ViewSequence; N]");

        let index: usize = start.routing_id().try_into().unwrap();
        // We know the index is in bounds because it was created from an index into a value of Self
        let (this_skip, inner_state) = &mut seq_state[index];
        elements.skip(*this_skip);
        self[index].seq_message(inner_state, message, elements, app_state)
    }

    #[doc(hidden)]
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        for (idx, (seq, (_, state))) in self.iter().zip(seq_state).enumerate() {
            ctx.with_id(
                ViewId::new(idx.try_into().expect(
                    "ViewSequence arrays with more than u64::MAX + 1 elements not supported",
                )),
                |ctx| {
                    seq.seq_teardown(state, ctx, elements);
                },
            );
        }
    }
}
