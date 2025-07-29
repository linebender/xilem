// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for sequences of views with a shared element type.

use alloc::vec::{Drain, Vec};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::element::NoElement;
use crate::{
    MessageContext, MessageResult, SuperElement, View, ViewElement, ViewId, ViewMarker,
    ViewPathTracker,
};

/// An append only `Vec`.
///
/// This will be passed to [`ViewSequence::seq_build`] to
/// build the list of initial elements whilst materializing the sequence.
#[derive(Debug)]
pub struct AppendVec<T> {
    inner: Vec<T>,
}

impl<T> AppendVec<T> {
    /// Convert `self` into the underlying `Vec`
    #[must_use]
    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }
    /// Add an item to the end of the vector.
    pub fn push(&mut self, item: T) {
        self.inner.push(item);
    }
    /// [Drain](Vec::drain) all items from this `AppendVec`.
    pub fn drain(&mut self) -> Drain<'_, T> {
        self.inner.drain(..)
    }
    /// Equivalent to [`ElementSplice::index`].
    pub fn index(&self) -> usize {
        // If there are no items, to get here we need to skip 0
        // if there is one, we need to skip 1
        self.inner.len()
    }
    /// Returns `true` if the vector contains no elements.
    ///
    /// See [`Vec::is_empty`] for more details
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> From<Vec<T>> for AppendVec<T> {
    fn from(inner: Vec<T>) -> Self {
        Self { inner }
    }
}

impl<T> Default for AppendVec<T> {
    fn default() -> Self {
        Self {
            inner: Vec::default(),
        }
    }
}

// --- MARK: Traits

/// Views for ordered sequences of elements.
///
/// Generally, a container view will internally contain a `ViewSequence`.
/// The child elements of the container will be updated by the `ViewSequence`.
///
/// This is implemented for:
///  - Any single [`View`], where the view's element type
///    is [compatible](SuperElement) with the sequence's element type.
///    This is the root implementation, by which the sequence actually
///    updates the relevant element.
///  - An `Option` of a `ViewSequence` value.
///    The elements of the inner sequence will be inserted into the
///    sequence if the value is `Some`, and removed once the value is `None`.
///  - A [`Vec`] of `ViewSequence` values.
///    Note that this will have persistent allocation with size proportional
///    to the *longest* `Vec` which is ever provided in the View tree, as this
///    uses a generational indexing scheme.
///  - An [`array`] of `ViewSequence` values.
///  - Tuples of `ViewSequences` with up to 15 elements.
///    These can be nested if an ad-hoc sequence of more than 15 sequences is needed.
pub trait ViewSequence<State, Action, Context, Element>: 'static
where
    Context: ViewPathTracker,
    Element: ViewElement,
{
    /// The associated state of this sequence. The main purposes of this are:
    /// - To store generations and other data needed to avoid routing stale messages
    ///   to incorrect views.
    /// - To pass on the state of child sequences, or a child View's [`ViewState`].
    ///
    /// The type used for this associated type cannot be treated as public API; this is
    /// internal state to the `ViewSequence` implementation.
    /// That is, `ViewSequence` implementations are permitted to change the type they use for this
    ///  during even a patch release of their crate.
    ///
    /// [`ViewState`]: View::ViewState
    type SeqState;

    /// Build the associated widgets into `elements` and initialize all states.
    #[must_use]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
    ) -> Self::SeqState;

    /// Update the associated widgets.
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    );

    /// Update the associated widgets.
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    );

    /// Propagate a message.
    ///
    /// Handle a message, propagating to child views if needed.
    /// The context contains both the path of this view, and the remaining
    /// path that the rest of the implementation needs to go along.
    /// The first items in the remaining part of thtis path will be those added
    /// in build and/or rebuild.
    ///
    /// The provided `elements` must be at the same [`index`](ElementSplice::index)
    /// it was at when `rebuild` (or `build`) was last called on this sequence.
    ///
    /// The easiest way to achieve this is to cache the index reached before any child
    /// sequence's build/rebuild, and skip to that value.
    /// Note that the amount you will need to skip to reach this value won't be the index
    /// directly, but instead must be the difference between this index and the value of
    /// the index at the start of your build/rebuild.
    // Potential optimisation: Sequence implementations can be grouped into three classes:
    // 1) Statically known size (e.g. a single element, a tuple with only statically known size)
    // 2) Linear time known size (e.g. a tuple of linear or better known size, a vec of statically known size, an option)
    // 3) Dynamically known size (e.g. a vec)
    // For case 1 and maybe case 2, we don't need to store the indices, and could instead rebuild them dynamically.
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

/// A temporary "splice" to add, update and delete in an (ordered) sequence of elements.
/// It is mainly intended for view sequences.
pub trait ElementSplice<Element: ViewElement> {
    /// Run a function with access to the associated [`AppendVec`].
    ///
    /// Each element [pushed](AppendVec::push) to the provided vector will be logically
    /// [inserted](ElementSplice::insert) into `self`.
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<Element>) -> R) -> R;
    /// Insert a new element at the current index in the resulting collection.
    fn insert(&mut self, element: Element);
    /// Mutate the next existing element.
    fn mutate<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
    /// Don't make any changes to the next n existing elements.
    fn skip(&mut self, n: usize);
    /// How many elements you would need to [`skip`](ElementSplice::skip) from when this
    /// `ElementSplice` was created to get to the current element.
    ///
    /// Note that in using this function, previous views will have skipped.
    /// Values obtained from this method may change during any `rebuild`, but will not change
    /// between `build`/`rebuild` and the next `message`
    fn index(&self) -> usize;
    /// Delete the next existing element, after running a function on it.
    fn delete<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
}

// --- MARK: For V: View

impl<State, Action, Context, V, Element> ViewSequence<State, Action, Context, Element> for V
where
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ViewMarker,
    Element: SuperElement<V::Element, Context>,
    V::Element: ViewElement,
{
    type SeqState = V::ViewState;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
    ) -> Self::SeqState {
        let (element, view_state) = self.build(ctx, app_state);
        elements.push(Element::upcast(ctx, element));
        view_state
    }
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        // Mutate the item we added in `seq_build`
        elements.mutate(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.rebuild(prev, seq_state, ctx, element, app_state);
            });
        });
    }
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        elements.delete(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.teardown(seq_state, ctx, element, app_state);
            });
        });
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        elements.mutate(|this_element| {
            Element::with_downcast_val(this_element, |element| {
                self.message(seq_state, message, element, app_state)
            })
            .1
        })
    }
}

// --- MARK: for Option<Seq>

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
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = OptionSeqState<Seq::SeqState>;

    #[doc(hidden)]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
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
        app_state: &mut State,
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
                    prev.seq_teardown(inner_state, ctx, elements, app_state);
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
        app_state: &mut State,
    ) {
        assert_eq!(
            self.is_some(),
            seq_state.inner.is_some(),
            "Inconsistent ViewSequence state. Perhaps the parent is mixing up children"
        );
        if let Some((seq, inner_state)) = self.as_ref().zip(seq_state.inner.as_mut()) {
            ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                seq.seq_teardown(inner_state, ctx, elements, app_state);
            });
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
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

// --- MARK: for Vec<Seq>

/// The state used to implement `ViewSequence` for `Vec<impl ViewSequence>`
///
/// We use a generation arena for vector types, with half of the `ViewId` dedicated
/// to the index, and the other half used for the generation.
///
// This is managed in [`create_generational_view_id`] and [`view_id_to_index_generation`]
#[doc(hidden)]
#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
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
    #![allow(clippy::cast_possible_truncation)]
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
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = VecViewState<Seq::SeqState>;

    #[doc(hidden)]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
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
                let inner_state = ctx.with_id(id, |ctx| seq.seq_build(ctx, elements, app_state));
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
        app_state: &mut State,
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
                child.seq_rebuild(child_prev, child_state, ctx, elements, app_state);
            });
        }
        let n = self.len();
        let prev_n = prev.len();
        #[allow(clippy::comparison_chain)]
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
                    old_seq.seq_teardown(&mut inner_state, ctx, elements, app_state);
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
                            let inner_state =
                                ctx.with_id(id, |ctx| seq.seq_build(ctx, elements, app_state));
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
        app_state: &mut State,
    ) {
        for (index, ((seq, (_, state)), generation)) in self
            .iter()
            .zip(&mut seq_state.inner_states)
            .zip(&seq_state.generations)
            .enumerate()
        {
            let id = create_generational_view_id(index, *generation);
            ctx.with_id(id, |ctx| seq.seq_teardown(state, ctx, elements, app_state));
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for Vec<ViewSequence>");
        let (index, generation) = view_id_to_index_generation(start);
        let stored_generation = &seq_state.generations[index];
        if *stored_generation != generation {
            // The value in the sequence i
            return MessageResult::Stale;
        }
        // Panics if index is out of bounds, but we know it isn't because this is the same generation
        let (child_skip, inner_state) = &mut seq_state.inner_states[index];

        elements.skip(*child_skip);
        self[index].seq_message(inner_state, message, elements, app_state)
    }
}

// --- MARK: for [Seq; N]

impl<State, Action, Context, Element, Seq, const N: usize>
    ViewSequence<State, Action, Context, Element> for [Seq; N]
where
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    /// The fields of the tuple are (number of widgets to skip, child `SeqState`).
    type SeqState = [(usize, Seq::SeqState); N];

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
        message: &mut MessageContext,
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
        app_state: &mut State,
    ) {
        for (idx, (seq, (_, state))) in self.iter().zip(seq_state).enumerate() {
            ctx.with_id(
                ViewId::new(idx.try_into().expect(
                    "ViewSequence arrays with more than u64::MAX + 1 elements not supported",
                )),
                |ctx| {
                    seq.seq_teardown(state, ctx, elements, app_state);
                },
            );
        }
    }
}

// --- MARK: for ()

impl<State, Action, Context, Element> ViewSequence<State, Action, Context, Element> for ()
where
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = ();

    fn seq_build(
        &self,
        _: &mut Context,
        _: &mut AppendVec<Element>,
        _: &mut State,
    ) -> Self::SeqState {
    }

    fn seq_rebuild(
        &self,
        _: &Self,
        _: &mut Self::SeqState,
        _: &mut Context,
        _: &mut impl ElementSplice<Element>,
        _: &mut State,
    ) {
    }

    fn seq_teardown(
        &self,
        _seq_state: &mut Self::SeqState,
        _ctx: &mut Context,
        _elements: &mut impl ElementSplice<Element>,
        _: &mut State,
    ) {
    }

    fn seq_message(
        &self,
        _: &mut Self::SeqState,
        message: &mut MessageContext,
        _: &mut impl ElementSplice<Element>,
        _: &mut State,
    ) -> MessageResult<Action> {
        unreachable!("Messages should never be dispatched to an empty tuple {message:?}.");
    }
}

// --- MARK: for (Seq,)

impl<State, Action, Context, Element, Seq> ViewSequence<State, Action, Context, Element> for (Seq,)
where
    Seq: ViewSequence<State, Action, Context, Element>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = Seq::SeqState;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: &mut State,
    ) -> Self::SeqState {
        self.0.seq_build(ctx, elements, app_state)
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        self.0
            .seq_rebuild(&prev.0, seq_state, ctx, elements, app_state);
    }

    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        self.0.seq_teardown(seq_state, ctx, elements, app_state);
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
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
                State,
                Action,
                Context: ViewPathTracker,
                Element: ViewElement,
                $($seq: ViewSequence<State, Action, Context, Element>,)+
            > ViewSequence<State, Action, Context, Element> for ($($seq,)+)

        {
            /// The fields of the inner tuples are (number of widgets to skip, child state).
            type SeqState = ($((usize, $seq::SeqState),)+);

            fn seq_build(
                &self,
                ctx: &mut Context,
                elements: &mut AppendVec<Element>,
                app_state: &mut State,
            ) -> Self::SeqState {
                let start_idx = elements.index();
                ($(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        let this_skip = elements.index() - start_idx;
                        let state = self.$idx.seq_build(ctx, elements, app_state);
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
                app_state: &mut State,
            ) {
                let start_idx = elements.index();
                $(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        seq_state.$idx.0 = elements.index() - start_idx;
                        self.$idx.seq_rebuild(&prev.$idx, &mut seq_state.$idx.1, ctx, elements, app_state);
                    });
                )+
            }

            fn seq_teardown(
                &self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
                app_state: &mut State,
            ) {
                $(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        self.$idx.seq_teardown(&mut seq_state.$idx.1, ctx, elements, app_state)
                    });
                )+
            }

            fn seq_message(
                &self,
                seq_state: &mut Self::SeqState,
                message: &mut MessageContext,
                elements: &mut impl ElementSplice<Element>,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                let start = message
                    .take_first()
                    .expect("Id path has elements for tuple");
                match start.routing_id() {
                    $(
                        $idx => {
                            elements.skip(seq_state.$idx.0);
                            self.$idx.seq_message(&mut seq_state.$idx.1, message, elements, app_state)
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

// --- MARK: NoElements

/// A stub `ElementSplice` implementation for `NoElement`.
///
/// It is technically possible for someone to create an implementation of `ViewSequence`
/// which uses a (different) `NoElement` `ElementSplice`. But we don't think that sequence could be meaningful.
pub(crate) struct NoElements;

impl ElementSplice<NoElement> for NoElements {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<NoElement>) -> R) -> R {
        let mut append_vec = AppendVec::default();
        f(&mut append_vec)
    }

    fn insert(&mut self, _: NoElement) {}

    fn mutate<R>(&mut self, f: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        f(())
    }

    fn skip(&mut self, _: usize) {}

    fn index(&self) -> usize {
        0
    }

    fn delete<R>(&mut self, f: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        f(())
    }
}

/// The [`ViewSequence`] for [`without_elements`], see its documentation for more context.
#[derive(Debug)]
pub struct WithoutElements<Seq, State, Action, Context> {
    seq: Seq,
    phantom: PhantomData<fn() -> (State, Action, Context)>,
}

/// An adapter which turns a [`ViewSequence`] containing any number of views with side effects, into a `ViewSequence` with any element type.
///
/// The returned `ViewSequence` will not contain any elements, and will instead run the side effects
/// of the wrapped `ViewSequence` when built and/or rebuilt.
///
/// This can be used to embed side-effects naturally into the flow of your program.
/// This can be used as an alternative to [`fork`](crate::fork) , which avoids adding extra nesting at the cost of only being usable in places where there is already an existing sequence.
///
/// # Examples
///
/// ```
/// # use xilem_core::docs::{DocsViewSequence as WidgetViewSequence, some_component_generic as component};
/// use xilem_core::{without_elements, run_once};
///
/// fn isolated_child(state: &mut AppState) -> impl WidgetViewSequence<AppState> {
///     (component(state), without_elements(run_once(|| {})))
/// }
///
/// # struct AppState;
/// ```
pub fn without_elements<State, Action, Context, Seq>(
    seq: Seq,
) -> WithoutElements<Seq, State, Action, Context>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker + 'static,
    Seq: ViewSequence<State, Action, Context, NoElement>,
{
    WithoutElements {
        seq,
        phantom: PhantomData,
    }
}

impl<State, Action, Context, Element, Seq> ViewSequence<State, Action, Context, Element>
    for WithoutElements<Seq, State, Action, Context>
where
    State: 'static,
    Action: 'static,
    Element: ViewElement,
    Context: ViewPathTracker + 'static,
    Seq: ViewSequence<State, Action, Context, NoElement>,
{
    type SeqState = Seq::SeqState;

    fn seq_build(
        &self,
        ctx: &mut Context,
        _elements: &mut AppendVec<Element>,
        app_state: &mut State,
    ) -> Self::SeqState {
        self.seq
            .seq_build(ctx, &mut AppendVec::default(), app_state)
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        _elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        self.seq
            .seq_rebuild(&prev.seq, seq_state, ctx, &mut NoElements, app_state);
    }

    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        _elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) {
        self.seq
            .seq_teardown(seq_state, ctx, &mut NoElements, app_state);
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        _elements: &mut impl ElementSplice<Element>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.seq
            .seq_message(seq_state, message, &mut NoElements, app_state)
    }
}
