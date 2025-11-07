// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppendVec, Arg, Count, ElementSplice, MessageContext, MessageResult, ViewArgument, ViewElement,
    ViewId, ViewPathTracker, ViewSequence,
};

/// The state used to implement `ViewSequence` for `Option<impl ViewSequence>`
#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
#[derive(Debug)]
pub struct OptionSeqState<InnerState> {
    /// The current state.
    ///
    /// Will be `None` if the previous value was `None`.
    inner: Option<InnerState>,
    /// The generation this option is at.
    ///
    /// If the inner sequence was `Some`, then `None`, then `Some`, the sequence
    /// is treated as a new sequence, as e.g. build has been called again.
    generation: u64,
}

/// The implementation for an `Option` of a `ViewSequence`.
///
/// Will mark messages which were sent to a `Some` value if a `None` has since
/// occurred as stale.
impl<State, Action, Context, Element, Seq> ViewSequence<State, Action, Context, Element>
    for Option<Seq>
where
    State: ViewArgument,
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = OptionSeqState<Seq::SeqState>;

    #[doc(hidden)]
    const ELEMENTS_COUNT: Count = const {
        match Seq::ELEMENTS_COUNT {
            // This sequence has zero or one children,
            // which is best explained as "Many".
            Count::One => Count::Many,
            Count::Many => Count::Many,
            Count::Unknown => Count::Unknown,
            Count::Zero => Count::Zero,
        }
    };

    #[doc(hidden)]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: Arg<'_, State>,
    ) -> Self::SeqState {
        let generation = 0;
        match self {
            Some(seq) => {
                let inner = ctx.with_id(ViewId::new(generation), |ctx| {
                    seq.seq_build(ctx, elements, app_state)
                });
                OptionSeqState {
                    inner: Some(inner),
                    generation,
                }
            }
            None => OptionSeqState {
                inner: None,
                generation,
            },
        }
    }

    #[doc(hidden)]
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) {
        // If `prev` was `Some`, we set `seq_state` in reacting to it (and building the inner view)
        // This could only fail if some malicious parent view was messing with our internal state
        // (i.e. mixing up the state from different instances)
        assert_eq!(
            prev.is_some(),
            seq_state.inner.is_some(),
            "Inconsistent ViewSequence state. Perhaps the parent is mixing up children"
        );
        match (self, prev.as_ref().zip(seq_state.inner.as_mut())) {
            (None, None) => {
                // Nothing to do, there is no corresponding element
            }
            (Some(seq), Some((prev, inner_state))) => {
                // Perform a normal rebuild
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    seq.seq_rebuild(prev, inner_state, ctx, elements, app_state);
                });
            }
            (Some(seq), None) => {
                // The sequence is newly re-added, build the inner sequence
                // We don't increment the generation here, as that was already done in the below case
                let inner_state = ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    elements.with_scratch(|elements| seq.seq_build(ctx, elements, app_state))
                });
                seq_state.inner = Some(inner_state);
            }
            (None, Some((prev, inner_state))) => {
                // Run teardown with the old path
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    prev.seq_teardown(inner_state, ctx, elements);
                });
                // The sequence has just been destroyed, teardown the old view
                // We increment the generation only on the falling edge by convention
                // This choice has no impact on functionality
                seq_state.inner = None;

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                seq_state.generation = seq_state.generation.wrapping_add(1);
            }
        }
    }

    #[doc(hidden)]
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        assert_eq!(
            self.is_some(),
            seq_state.inner.is_some(),
            "Inconsistent ViewSequence state. Perhaps the parent is mixing up children"
        );
        if let Some((seq, inner_state)) = self.as_ref().zip(seq_state.inner.as_mut()) {
            ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                seq.seq_teardown(inner_state, ctx, elements);
            });
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for Option<ViewSequence>");

        if start.routing_id() != seq_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale;
        }
        assert_eq!(
            self.is_some(),
            seq_state.inner.is_some(),
            "Inconsistent ViewSequence state. Perhaps the parent is mixing up children"
        );
        if let Some((seq, inner_state)) = self.as_ref().zip(seq_state.inner.as_mut()) {
            seq.seq_message(inner_state, message, elements, app_state)
        } else {
            // TODO: this should be unreachable as the generation was increased on the falling edge
            MessageResult::Stale
        }
    }
}
