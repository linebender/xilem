use std::num::NonZeroU64;

use masonry::{widget::WidgetMut, Widget, WidgetPod};

use crate::{ChangeFlags, MasonryView, MessageResult, ViewCx, ViewId};

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
    ) -> ChangeFlags;

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
    ) -> ChangeFlags {
        let mut element = elements.mutate();
        let downcast = element.downcast::<View::Element>();

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
        match self {
            Some(this) => OptionSeqState {
                inner: Some(this.build(cx, elements)),
                generation: 0,
            },
            None => OptionSeqState {
                inner: None,
                generation: 0,
            },
        }
    }

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        match (self, prev) {
            (Some(this), Some(prev)) => this.rebuild(todo!("DJMcNab"), cx, prev, elements),
            (None, Some(prev)) => {
                let count = prev.count();
                elements.delete(count);

                ChangeFlags::CHANGED
            }
            (Some(this), None) => {
                // TODO: Assign an increased generation ViewId here.
                this.build(cx, elements);
                ChangeFlags::CHANGED
            }
            (None, None) => ChangeFlags::UNCHANGED,
        }
    }

    fn message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        if let Some(this) = self {
            this.message(todo!("DJMcNab"), id_path, message, app_state)
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
        let inner = self.iter().enumerate().map(|(i, child)| {
            let i: u64 = i.try_into().unwrap();
            let id = NonZeroU64::new(i + 1).unwrap();
            cx.with_id(ViewId::for_type::<VT>(id), |cx| child.build(cx, elements))
        });
        let inner_with_generations = inner.map(|it| (it, 0)).collect();
        VecViewState {
            global_generation: 0,
            inner_with_generations,
        }
    }

    fn rebuild(
        &self,
        seq_state: &mut Self::SeqState,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::UNCHANGED;
        for (i, (child, child_prev)) in self.iter().zip(prev).enumerate() {
            // TODO: Do we want these ids to (also?) have a generational component?
            let i: u64 = i.try_into().unwrap();
            let id = NonZeroU64::new(i + 1).unwrap();
            cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                let el_changed = child.rebuild(todo!("DJMcNab"), cx, child_prev, elements);
                changed.changed |= el_changed.changed;
            });
        }
        let n = self.len();
        if n < prev.len() {
            let n_delete = prev[n..].iter().map(ViewSequence::count).sum();
            elements.delete(n_delete);
            changed.changed |= ChangeFlags::CHANGED.changed;
        } else if n > prev.len() {
            // This suggestion from clippy is kind of bad, because we use the absolute index in the id
            #[allow(clippy::needless_range_loop)]
            for ix in prev.len()..n {
                let id_u64: u64 = ix.try_into().unwrap();
                let id = NonZeroU64::new(id_u64 + 1).unwrap();
                cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                    self[ix].build(cx, elements);
                });
            }
            changed.changed |= ChangeFlags::CHANGED.changed;
        }
        changed
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
        let index_plus_one: usize = start.routing_id().get().try_into().unwrap();
        self[index_plus_one - 1].message(
            &mut seq_state.inner_with_generations[index_plus_one - 1].0,
            rest,
            message,
            app_state,
        )
    }

    fn count(&self) -> usize {
        self.iter().map(ViewSequence::count).sum()
    }
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
    ) -> ChangeFlags {
        ChangeFlags::UNCHANGED
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
    ) -> ChangeFlags {
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

const BASE_ID: NonZeroU64 = match NonZeroU64::new(1) {
    Some(it) => it,
    None => unreachable!(),
};

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
                    cx.with_id(ViewId::for_type::<$seq>(BASE_ID.saturating_add($idx)), |cx| {
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
            ) -> ChangeFlags {
                let mut flags = ChangeFlags::UNCHANGED;
                $(
                    cx.with_id(ViewId::for_type::<$seq>(BASE_ID.saturating_add($idx)), |cx| {
                        flags.changed |= self.$idx.rebuild(&mut seq_state.$idx, cx, &prev.$idx, elements).changed;
                    });
                )+
                flags
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
                let index_plus_one = start.routing_id().get();
                match index_plus_one - 1 {
                    $(
                        $idx => self.$idx.message(&mut seq_state.$idx, rest, message, app_state),
                    )+
                    // TODO: Should not panic? Is this a dynamic viewsequence thing?
                    _ => unreachable!("Unexpected id path {start:?} in tuple"),
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
