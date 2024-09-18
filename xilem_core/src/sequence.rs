// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for sequences of views with a shared element type.

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use alloc::vec::Drain;
use alloc::vec::Vec;

// use crate::element::NoElement;
use crate::{
    DynMessage, MessageResult, SuperElement, View, ViewElement, ViewId, ViewMarker, ViewPathTracker,
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
///
pub trait ViewSequence<State, Action, Context, Element, Message = DynMessage>: 'static
where
    Context: ViewPathTracker,
    Element: ViewElement,
{
    /// The associated state of this sequence. The main purposes of this are:
    /// - To store generations and other data needed to avoid routing stale messages
    ///   to incorrect views.
    /// - To pass on the state of child sequences, or a child View's [`ViewState`].
    ///
    /// [`ViewState`]: View::ViewState
    type SeqState;

    /// Build the associated widgets into `elements` and initialize all states.
    #[must_use]
    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState;

    /// Update the associated widgets.
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    );

    /// Update the associated widgets.
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    );

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids, where the first item identifiers a child element of this sequence, if necessary.
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message>;
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
    /// Delete the next existing element, after running a function on it.
    fn delete<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
}

impl<State, Action, Context, V, Element, Message>
    ViewSequence<State, Action, Context, Element, Message> for V
where
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message> + ViewMarker,
    Element: SuperElement<V::Element, Context>,
    V::Element: ViewElement,
{
    type SeqState = V::ViewState;

    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        let (element, view_state) = self.build(ctx);
        elements.push(Element::upcast(ctx, element));
        view_state
    }
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        // Mutate the item we added in `seq_build`
        elements.mutate(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.rebuild(prev, seq_state, ctx, element);
            });
        });
    }
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        elements.delete(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.teardown(seq_state, ctx, element);
            });
        });
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        self.message(seq_state, id_path, message, app_state)
    }
}

/// The state used to implement `ViewSequence` for `Option<impl ViewSequence>`
#[allow(unnameable_types)] // Public because of trait visibility rules, but has no public API.
pub struct OptionSeqState<InnerState> {
    /// The current state.
    ///
    /// Will be `None` if the previous value was `None`.
    inner: Option<InnerState>,
    /// The generation this option is at.
    ///
    /// If the inner sequence was Some, then None, then Some, the sequence
    /// is treated as a new sequence, as e.g. build has been called again.
    generation: u64,
}

/// The implementation for an `Option` of a `ViewSequence`.
///
/// Will mark messages which were sent to a `Some` value if a `None` has since
/// occurred as stale.
impl<State, Action, Context, Element, Seq, Message>
    ViewSequence<State, Action, Context, Element, Message> for Option<Seq>
where
    Seq: ViewSequence<State, Action, Context, Element, Message>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = OptionSeqState<Seq::SeqState>;

    #[doc(hidden)]
    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        let generation = 0;
        match self {
            Some(seq) => {
                let inner =
                    ctx.with_id(ViewId::new(generation), |ctx| seq.seq_build(ctx, elements));
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
    ) {
        // If `prev` was `Some`, we set `seq_state` in reacting to it (and building the inner view)
        // This could only fail if some malicious parent view was messing with our internal state
        // (i.e. mixing up the state from different instances)
        assert_eq!(prev.is_some(), seq_state.inner.is_some());
        match (self, prev.as_ref().zip(seq_state.inner.as_mut())) {
            (None, None) => {
                // Nothing to do, there is no corresponding element
            }
            (Some(seq), Some((prev, inner_state))) => {
                // Perform a normal rebuild
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    seq.seq_rebuild(prev, inner_state, ctx, elements);
                });
            }
            (Some(seq), None) => {
                // The sequence is newly re-added, build the inner sequence
                // We don't increment the generation here, as that was already done in the below case
                let inner_state = ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    elements.with_scratch(|elements| seq.seq_build(ctx, elements))
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
        assert_eq!(self.is_some(), seq_state.inner.is_some());
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
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for Option<ViewSequence>");
        if start.routing_id() != seq_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        assert_eq!(self.is_some(), seq_state.inner.is_some());
        if let Some((seq, inner_state)) = self.as_ref().zip(seq_state.inner.as_mut()) {
            seq.seq_message(inner_state, rest, message, app_state)
        } else {
            // TODO: this should be unreachable as the generation was increased on the falling edge
            MessageResult::Stale(message)
        }
    }
}

/// The state used to implement `ViewSequence` for `Vec<impl ViewSequence>`
///
/// We use a generation arena for vector types, with half of the `ViewId` dedicated
/// to the index, and the other half used for the generation.
///
// This is managed in [`create_vector_view_id`] and [`view_id_to_index_generation`]
#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct VecViewState<InnerState> {
    inner_states: Vec<InnerState>,

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

/// Undoes [`create_vector_view_id`]
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
impl<State, Action, Context, Element, Seq, Message>
    ViewSequence<State, Action, Context, Element, Message> for Vec<Seq>
where
    Seq: ViewSequence<State, Action, Context, Element, Message>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    // We hide all the items in these implementation so that the top-level
    // comment is always shown. This lets us explain the caveats.
    #[doc(hidden)]
    type SeqState = VecViewState<Seq::SeqState>;

    #[doc(hidden)]
    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        let generations = alloc::vec![0; self.len()];
        let inner_states = self
            .iter()
            .enumerate()
            .zip(&generations)
            .map(|((index, seq), generation)| {
                let id = create_generational_view_id(index, *generation);
                ctx.with_id(id, |ctx| seq.seq_build(ctx, elements))
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
    ) {
        for (i, (((child, child_prev), child_state), child_generation)) in self
            .iter()
            .zip(prev)
            .zip(&mut seq_state.inner_states)
            .zip(&seq_state.generations)
            .enumerate()
        {
            // Rebuild the items which are common to both vectors
            let id = create_generational_view_id(i, *child_generation);
            ctx.with_id(id, |ctx| {
                child.seq_rebuild(child_prev, child_state, ctx, elements);
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
            for (index, ((old_seq, generation), mut inner_state)) in
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
                    // Note that we have a slightly different strategy to Bevy, where we use a global generation
                    // This theoretically allows some of the memory in `seq_state` to be reclaimed, at the cost of making overflow
                    // more likely here. Note that we don't actually reclaim this memory at the moment.

                    // We use 0 to wrap around. It would require extremely unfortunate timing to get an async event
                    // with the correct generation exactly u32::MAX generations late, so wrapping is the best option
                    0
                });
            }
        } else if n > prev_n {
            // If needed, create new generations
            seq_state.generations.resize(n, 0);
            elements.with_scratch(|elements| {
                seq_state.inner_states.extend(
                    self[prev_n..]
                        .iter()
                        .zip(&seq_state.generations[prev_n..])
                        .enumerate()
                        .map(|(index, (seq, generation))| {
                            let id = create_generational_view_id(index + prev_n, *generation);
                            ctx.with_id(id, |ctx| seq.seq_build(ctx, elements))
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
        for (index, ((seq, state), generation)) in self
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
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for Vec<ViewSequence>");
        let (index, generation) = view_id_to_index_generation(*start);
        let stored_generation = &seq_state.generations[index];
        if *stored_generation != generation {
            // The value in the sequence i
            return MessageResult::Stale(message);
        }
        // Panics if index is out of bounds, but we know it isn't because this is the same generation
        let inner_state = &mut seq_state.inner_states[index];
        self[index].seq_message(inner_state, rest, message, app_state)
    }
}

impl<State, Action, Context, Element, Seq, Message, const N: usize>
    ViewSequence<State, Action, Context, Element, Message> for [Seq; N]
where
    Seq: ViewSequence<State, Action, Context, Element, Message>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = [Seq::SeqState; N];

    #[doc(hidden)]
    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        // there's no enumerate directly on an array
        let mut idx = 0;
        self.each_ref().map(|vs| {
            let state = ctx.with_id(ViewId::new(idx), |ctx| vs.seq_build(ctx, elements));
            idx += 1;
            state
        })
    }

    #[doc(hidden)]
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        for (idx, ((seq, prev_seq), state)) in self.iter().zip(prev).zip(seq_state).enumerate() {
            ctx.with_id(
                ViewId::new(idx.try_into().expect(
                    "ViewSequence arrays with more than u64::MAX + 1 elements not supported",
                )),
                |ctx| {
                    seq.seq_rebuild(prev_seq, state, ctx, elements);
                },
            );
        }
    }

    #[doc(hidden)]
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for [ViewSequence; N]");

        let index: usize = start.routing_id().try_into().unwrap();
        // We know the index is in bounds because it was created from an index into a value of Self
        let inner_state = &mut seq_state[index];
        self[index].seq_message(inner_state, rest, message, app_state)
    }

    #[doc(hidden)]
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        for (idx, (seq, state)) in self.iter().zip(seq_state).enumerate() {
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

impl<State, Action, Context, Element, Message>
    ViewSequence<State, Action, Context, Element, Message> for ()
where
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = ();

    fn seq_build(&self, _: &mut Context, _: &mut AppendVec<Element>) -> Self::SeqState {}

    fn seq_rebuild(
        &self,
        _: &Self,
        _: &mut Self::SeqState,
        _: &mut Context,
        _: &mut impl ElementSplice<Element>,
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
        _: &[ViewId],
        _message: Message,
        _: &mut State,
    ) -> MessageResult<Action, Message> {
        unreachable!("Messages should never be dispatched to an empty tuple");
        // TODO add Debug trait bound because of this?: , got {message:?}
    }
}

impl<State, Action, Context, Element, Seq, Message>
    ViewSequence<State, Action, Context, Element, Message> for (Seq,)
where
    Seq: ViewSequence<State, Action, Context, Element, Message>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = Seq::SeqState;

    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        self.0.seq_build(ctx, elements)
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        self.0.seq_rebuild(&prev.0, seq_state, ctx, elements);
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
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        self.0.seq_message(seq_state, id_path, message, app_state)
    }
}

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
                $($seq: ViewSequence<State, Action, Context, Element, Message>,)+
                Message,
            > ViewSequence<State, Action, Context, Element, Message> for ($($seq,)+)

        {
            type SeqState = ($($seq::SeqState,)+);

            fn seq_build(
                &self,
                ctx: &mut Context,
                elements: &mut AppendVec<Element>,
            ) -> Self::SeqState {
                ($(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        self.$idx.seq_build(ctx, elements)
                    }),
                )+)
            }

            fn seq_rebuild(
                &self,
                prev: &Self,
                seq_state: &mut Self::SeqState,
                ctx: &mut Context,
                elements: &mut impl ElementSplice<Element>,
            ) {
                $(
                    ctx.with_id(ViewId::new($idx), |ctx| {
                        self.$idx.seq_rebuild(&prev.$idx, &mut seq_state.$idx, ctx, elements);
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
                        self.$idx.seq_teardown(&mut seq_state.$idx, ctx, elements)
                    });
                )+
            }

            fn seq_message(
                &self,
                seq_state: &mut Self::SeqState,
                id_path: &[ViewId],
                message: Message,
                app_state: &mut State,
            ) -> MessageResult<Action, Message> {
                let (start, rest) = id_path
                    .split_first()
                    .expect("Id path has elements for tuple");
                match start.routing_id() {
                    $(
                        $idx => self.$idx.seq_message(&mut seq_state.$idx, rest, message, app_state),
                    )+
                    // If we have received a message, our parent is (mostly) certain that we requested it
                    // The only time that wouldn't be the case is when a generational index has overflowed?
                    _ => unreachable!("Unexpected id path {start:?} in tuple (wants to be routed via {rest:?})"),
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
