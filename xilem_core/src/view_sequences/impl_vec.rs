// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    AppendVec, Arg, Count, ElementSplice, MessageCtx, MessageResult, ViewArgument, ViewElement,
    ViewId, ViewPathTracker, ViewSequence,
};

/// The state used to implement `ViewSequence` for `Vec<impl ViewSequence>`
///
/// We use a generation arena for vector types, with half of the `ViewId` dedicated
/// to the index, and the other half used for the generation.
///
// This is managed in [`create_generational_view_id`] and [`view_id_to_index_generation`]
#[doc(hidden)]
#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
#[derive(Debug)]
pub struct VecViewState<InnerState> {
    // We use two vectors here because the `inner_states` is the
    // same length as the actual vector, whereas the generations
    // is the same length as the longest version of this we have seen.
    /// The fields of the tuple are (number of widgets to skip, `InnerState`).
    inner_states: Vec<(usize, InnerState)>,

    generations: Vec<u32>,
}

/// Turns an index and a generation into a packed id, suitable for use in
/// [`ViewId`]s
fn create_generational_view_id(index: usize, generation: u32) -> ViewId {
    let id_low: u32 = index
        .try_into()
        // If you're seeing this panic, you can use a nested `Vec<Vec<...>>`, where each individual `Vec`
        // has fewer than u32::MAX-1 elements.
        .expect("Views in a vector backed sequence must be indexable by u32");
    let id_low: u64 = id_low.into();
    let id_high: u64 = u64::from(generation) << 32;
    ViewId::new(id_high | id_low)
}

/// Undoes [`create_generational_view_id`]
fn view_id_to_index_generation(view_id: ViewId) -> (usize, u32) {
    #![expect(
        clippy::cast_possible_truncation,
        reason = "Explicitly splits u64 into two u32s"
    )]
    let view_id = view_id.routing_id();
    let id_low_ix = view_id as u32;
    let id_high_gen = (view_id >> 32) as u32;
    (id_low_ix as usize, id_high_gen)
}

/// The implementation for an `Vec` of a `ViewSequence`.
///
/// Will mark messages which were sent to any index as stale if
/// that index has been unused in the meantime.
impl<State, Action, Context, Element, Seq> ViewSequence<State, Action, Context, Element>
    for Vec<Seq>
where
    State: ViewArgument,
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = VecViewState<Seq::SeqState>;

    #[doc(hidden)]
    const ELEMENTS_COUNT: Count = Seq::ELEMENTS_COUNT.multiple();

    #[doc(hidden)]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        mut app_state: Arg<'_, State>,
    ) -> Self::SeqState {
        let start_idx = elements.index();
        let generations = alloc::vec![0; self.len()];
        let inner_states = self
            .iter()
            .enumerate()
            .zip(&generations)
            .map(|((index, seq), generation)| {
                let id = create_generational_view_id(index, *generation);
                let this_skip = elements.index() - start_idx;
                let inner_state = ctx.with_id(id, |ctx| {
                    seq.seq_build(ctx, elements, State::reborrow_mut(&mut app_state))
                });
                (this_skip, inner_state)
            })
            .collect();
        VecViewState {
            generations,
            inner_states,
        }
    }

    #[doc(hidden)]
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        mut app_state: Arg<'_, State>,
    ) {
        let start_idx = elements.index();
        for (i, (((child, child_prev), (child_skip, child_state)), child_generation)) in self
            .iter()
            .zip(prev)
            .zip(&mut seq_state.inner_states)
            .zip(&seq_state.generations)
            .enumerate()
        {
            *child_skip = elements.index() - start_idx;
            // Rebuild the items which are common to both vectors
            let id = create_generational_view_id(i, *child_generation);
            ctx.with_id(id, |ctx| {
                child.seq_rebuild(
                    child_prev,
                    child_state,
                    ctx,
                    elements,
                    State::reborrow_mut(&mut app_state),
                );
            });
        }
        let n = self.len();
        let prev_n = prev.len();
        if n < prev_n {
            let to_teardown = prev[n..].iter();
            // Keep the generations
            let generations = seq_state.generations[n..].iter_mut();
            // But remove the old states
            let states = seq_state.inner_states.drain(n..);
            for (index, ((old_seq, generation), (_, mut inner_state))) in
                to_teardown.zip(generations).zip(states).enumerate()
            {
                let id = create_generational_view_id(index + n, *generation);
                ctx.with_id(id, |ctx| {
                    old_seq.seq_teardown(&mut inner_state, ctx, elements);
                });
                // We increment the generation on the "falling edge" by convention
                *generation = generation.checked_add(1).unwrap_or_else(|| {
                    static SHOULD_WARN: AtomicBool = AtomicBool::new(true);
                    // We only want to warn about this once
                    // because e.g. if every item in a vector hits
                    // this at the same time, we don't want to repeat it too many times
                    if SHOULD_WARN.swap(false, Ordering::Relaxed) {
                        tracing::warn!(
                            inner_type = core::any::type_name::<Seq>(),
                            issue_url = "https://github.com/linebender/xilem/issues",
                            "Got overflowing generation in ViewSequence from `Vec<inner_type>`.\
                            This can possibly cause incorrect routing of async messages in extreme cases.\
                            Please open an issue if you see this. There are known solutions"
                        );
                    }
                    // The known solution mentioned in the above message is to use a different ViewId for the index and the generation
                    // We believe this to be superfluous for the default use case, as even with 1000 rebuilds a second, each adding
                    // to the same array, this would take 50 days of the application running continuously.
                    // See also https://github.com/bevyengine/bevy/pull/9907, where they warn in their equivalent case

                    // We use 0 to wrap around. It would require extremely unfortunate timing to get an async event
                    // with the correct generation exactly u32::MAX generations late, so wrapping is the best option
                    0
                });
            }
        } else if n > prev_n {
            // If needed, create new generations
            seq_state.generations.resize(n, 0);
            let outer_idx = elements.index();
            elements.with_scratch(|elements| {
                seq_state.inner_states.extend(
                    self[prev_n..]
                        .iter()
                        .zip(&seq_state.generations[prev_n..])
                        .enumerate()
                        .map(|(index, (seq, generation))| {
                            let id = create_generational_view_id(index + prev_n, *generation);
                            let this_skip = elements.index() + outer_idx - start_idx;
                            let inner_state = ctx.with_id(id, |ctx| {
                                seq.seq_build(ctx, elements, State::reborrow_mut(&mut app_state))
                            });
                            (this_skip, inner_state)
                        }),
                );
            });
        }
    }

    #[doc(hidden)]
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        for (index, ((seq, (_, state)), generation)) in self
            .iter()
            .zip(&mut seq_state.inner_states)
            .zip(&seq_state.generations)
            .enumerate()
        {
            let id = create_generational_view_id(index, *generation);
            ctx.with_id(id, |ctx| seq.seq_teardown(state, ctx, elements));
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageCtx,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for Vec<ViewSequence>");
        let (index, generation) = view_id_to_index_generation(start);
        let stored_generation = &seq_state.generations[index];
        if *stored_generation != generation {
            // The value in the sequence is no longer the same child
            return MessageResult::Stale;
        }
        // Panics if index is out of bounds, but we know it isn't because this is the same generation
        let (child_skip, inner_state) = &mut seq_state.inner_states[index];

        elements.skip(*child_skip);
        self[index].seq_message(inner_state, message, elements, app_state)
    }
}
