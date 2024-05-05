// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{widget::WidgetMut, Widget, WidgetPod};

use crate::{MasonryView, MessageResult, ViewCx, ViewId};

#[allow(clippy::len_without_is_empty)]
pub trait ElementSplice {
    /// Insert a new element at the current index in the resulting collection (and increment the index by 1)
    fn push(&mut self, element: WidgetPod<Box<dyn Widget>>);
    /// Mutate the next existing element, and add it to the resulting collection (and increment the index by 1)
    // TODO: This should actually return `WidgetMut<dyn Widget>`, but that isn't supported in Masonry itself yet
    fn mutate(&mut self) -> WidgetMut<Box<dyn Widget>>;
    /// Delete the next n existing elements (this doesn't change the index)
    fn delete(&mut self, n: usize);
    /// Current length of the elements collection
    // TODO: Is `len` needed?
    fn len(&self) -> usize;
}

/// This trait represents a (possibly empty) sequence of views.
///
/// It is up to the parent view how to lay out and display them.
pub trait ViewSequence<State, Action, Marker>: Send + 'static {
    type SeqState;
    // TODO: Rename to not overlap with MasonryView?
    /// Build the associated widgets and initialize all states.
    ///
    /// To be able to monitor changes (e.g. tree-structure tracking) rather than just adding elements,
    /// this takes an element splice as well (when it could be just a `Vec` otherwise)
    #[must_use]
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState;

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    );

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action>;

    /// Returns the current amount of widgets built by this sequence.
    fn count(&self) -> usize;
}

/// Workaround for trait ambiguity
///
/// These need to be public for type inference
#[doc(hidden)]
pub struct WasAView;
#[doc(hidden)]
/// See [`WasAView`]
pub struct WasASequence;

impl<State, Action, View: MasonryView<State, Action>> ViewSequence<State, Action, WasAView>
    for View
{
    type SeqState = View::ViewState;
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState {
        let (element, view_state) = self.build(cx);
        elements.push(element.boxed());
        view_state
    }

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) {
        let mut element = elements.mutate();
        let downcast = element.try_downcast::<View::Element>();

        if let Some(element) = downcast {
            self.rebuild(seq_state, cx, prev, element)
        } else {
            unreachable!("Tree structure tracking got wrong element type")
        }
    }

    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.message(seq_state, id_path, message, app_state)
    }

    fn count(&self) -> usize {
        1
    }
}

pub struct OptionSeqState<InnerState> {
    inner: Option<InnerState>,
    generation: u64,
}

impl<State, Action, Marker, VT: ViewSequence<State, Action, Marker>>
    ViewSequence<State, Action, (WasASequence, Marker)> for Option<VT>
{
    type SeqState = OptionSeqState<VT::SeqState>;
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState {
        let generation = 0;
        match self {
            Some(this) => {
                let inner = cx.with_id(ViewId::for_type::<VT>(generation), |cx| {
                    this.build(cx, elements)
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

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) {
        // If `prev` was `Some`, we set `seq_state` in reacting to it (and building the inner view)
        // This could only fail if some malicious parent view was messing with our internal state
        // (i.e. mixing up the state from different instances)
        debug_assert_eq!(prev.is_some(), seq_state.inner.is_some());
        match (self, prev.as_ref().zip(seq_state.inner.as_mut())) {
            (Some(this), Some((prev, prev_state))) => cx
                .with_id(ViewId::for_type::<VT>(seq_state.generation), |cx| {
                    this.rebuild(prev_state, cx, prev, elements)
                }),
            (None, Some((prev, _))) => {
                // Maybe replace with `prev.cleanup`?
                let count = prev.count();
                elements.delete(count);
                seq_state.inner = None;
                cx.mark_changed();
            }
            (Some(this), None) => {
                seq_state.generation += 1;
                let new_state = cx.with_id(ViewId::for_type::<VT>(seq_state.generation), |cx| {
                    Some(this.build(cx, elements))
                });
                seq_state.inner = new_state;
                cx.mark_changed();
            }
            (None, None) => (),
        }
    }

    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for Option<ViewSequence>");
        if start.routing_id() != seq_state.generation {
            return MessageResult::Stale(message);
        }
        debug_assert_eq!(self.is_some(), seq_state.inner.is_some());
        if let Some((this, seq_state)) = self.as_ref().zip(seq_state.inner.as_mut()) {
            this.message(seq_state, rest, message, app_state)
        } else {
            MessageResult::Stale(message)
        }
    }

    fn count(&self) -> usize {
        match self {
            Some(this) => this.count(),
            None => 0,
        }
    }
}

pub struct VecViewState<InnerState> {
    inner_with_generations: Vec<(InnerState, u32)>,
    global_generation: u32,
}

// TODO: We use raw indexing for this value. What would make it invalid?
impl<T, A, Marker, VT: ViewSequence<T, A, Marker>> ViewSequence<T, A, (WasASequence, Marker)>
    for Vec<VT>
{
    type SeqState = VecViewState<VT::SeqState>;
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState {
        let generation = 0;
        let inner = self.iter().enumerate().map(|(i, child)| {
            let id = create_vector_view_id(i, generation);

            cx.with_id(ViewId::for_type::<VT>(id), |cx| child.build(cx, elements))
        });
        let inner_with_generations = inner.map(|it| (it, generation)).collect();
        VecViewState {
            global_generation: generation,
            inner_with_generations,
        }
    }

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) {
        for (i, ((child, child_prev), (child_state, child_generation))) in self
            .iter()
            .zip(prev)
            .zip(&mut seq_state.inner_with_generations)
            .enumerate()
        {
            let id = create_vector_view_id(i, *child_generation);
            cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                child.rebuild(child_state, cx, child_prev, elements);
            });
        }
        let n = self.len();
        if n < prev.len() {
            let n_delete = prev[n..].iter().map(ViewSequence::count).sum();
            seq_state.inner_with_generations.drain(n..);
            elements.delete(n_delete);
            cx.mark_changed();
        } else if n > prev.len() {
            // Overflow condition: u32 incrementing by up to 1 per rebuild. Plausible if unlikely to overflow
            seq_state.global_generation = match seq_state.global_generation.checked_add(1) {
                Some(new_generation) => new_generation,
                None => {
                    // TODO: Inform the error
                    tracing::error!(
                        sequence_type = std::any::type_name::<VT>(),
                        issue_url = "https://github.com/linebender/xilem/issues",
                        "Got overflowing generation in ViewSequence. Please open an issue if you see this situation. There are known solutions"
                    );
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
                }
            };
            seq_state.inner_with_generations.reserve(n - prev.len());
            // This suggestion from clippy is kind of bad, because we use the absolute index in the id
            #[allow(clippy::needless_range_loop)]
            for ix in prev.len()..n {
                let id = create_vector_view_id(ix, seq_state.global_generation);
                let new_state = cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                    self[ix].build(cx, elements)
                });
                seq_state
                    .inner_with_generations
                    .push((new_state, seq_state.global_generation));
            }
            cx.mark_changed();
        }
    }

    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for vector");
        let (index, generation) = view_id_to_index_generation(start.routing_id());
        let (seq_state, stored_generation) = &mut seq_state.inner_with_generations[index];
        if *stored_generation != generation {
            return MessageResult::Stale(message);
        }
        self[index].message(seq_state, rest, message, app_state)
    }

    fn count(&self) -> usize {
        self.iter().map(ViewSequence::count).sum()
    }
}

/// Turns an index and a generation into a packed id, suitable for use in
/// [`ViewId`]s
fn create_vector_view_id(index: usize, generation: u32) -> u64 {
    let id_low: u32 = index.try_into().expect(
        "Can't have more than 4294967295 (u32::MAX-1) views in a single vector backed sequence",
    );
    let id_low: u64 = id_low.into();
    let id_high: u64 = u64::from(generation) << 32;
    id_high | id_low
}

/// Undoes [`create_vector_view_id`]
fn view_id_to_index_generation(view_id: u64) -> (usize, u32) {
    let id_low_ix = view_id as u32;
    let id_high_gen = (view_id >> 32) as u32;
    (id_low_ix as usize, id_high_gen)
}

impl<T, A> ViewSequence<T, A, ()> for () {
    type SeqState = ();
    fn build(&self, _: &mut ViewCx, _: &mut dyn ElementSplice) {}

    fn rebuild(
        &self,
        _seq_state: &mut Self::SeqState,
        _cx: &mut ViewCx,
        _prev: &Self,
        _elements: &mut dyn ElementSplice,
    ) {
    }

    fn message(
        &self,
        _seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        tracing::warn!(?id_path, "Dispatched message to empty tuple");
        MessageResult::Stale(message)
    }

    fn count(&self) -> usize {
        0
    }
}

impl<State, Action, M0, Seq0: ViewSequence<State, Action, M0>> ViewSequence<State, Action, (M0,)>
    for (Seq0,)
{
    type SeqState = Seq0::SeqState;
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState {
        self.0.build(cx, elements)
    }

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) {
        self.0.rebuild(seq_state, cx, &prev.0, elements)
    }

    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.0.message(seq_state, id_path, message, app_state)
    }

    fn count(&self) -> usize {
        self.0.count()
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
                $(
                    $marker,
                    $seq: ViewSequence<State, Action, $marker>,
                )+
            > ViewSequence<State, Action, ($($marker,)+)> for ($($seq,)+)
        {
            type SeqState = ($($seq::SeqState,)+);
            fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::SeqState {
                ($(
                    cx.with_id(ViewId::for_type::<$seq>($idx), |cx| {
                        self.$idx.build(cx, elements)
                    }),
                )+)
            }

            fn rebuild(
                &self,
                seq_state: &mut Self::SeqState,
                cx: &mut ViewCx,
                prev: &Self,
                elements: &mut dyn ElementSplice,
            ) {
                $(
                    cx.with_id(ViewId::for_type::<$seq>($idx), |cx| {
                        self.$idx.rebuild(&mut seq_state.$idx, cx, &prev.$idx, elements);
                    });
                )+
            }

            fn message(
                &self,
                seq_state: &mut Self::SeqState,
                id_path: &[ViewId],
                message: Box<dyn std::any::Any>,
                app_state: &mut State,
            ) -> MessageResult<Action> {
                let (start, rest) = id_path
                    .split_first()
                    .expect("Id path has elements for tuple");
                match start.routing_id() {
                    $(
                        $idx => self.$idx.message(&mut seq_state.$idx, rest, message, app_state),
                    )+
                    // If we have received a message, our parent is (mostly) certain that we requested it
                    // The only time that wouldn't be the case is when a generational index has overflowed?
                    _ => unreachable!("Unexpected id path {start:?} in tuple (wants to be routed via {rest:?})"),
                }
            }

            fn count(&self) -> usize {
                // Is there a way to do this which avoids the `+0`?
                $(self.$idx.count()+)+ 0
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
