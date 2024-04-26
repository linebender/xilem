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
    type State;
    /// Build the associated widgets and initialize all states.
    ///
    /// To be able to monitor changes (e.g. tree-structure tracking) rather than just adding elements,
    /// this takes an element splice as well (when it could be just a `Vec` otherwise)
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::State;

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        id_path: &[ViewId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action>;

    /// Returns the current amount of widgets built by this sequence.
    fn count(&self, state: &Self::State) -> usize;
}

/// Workaround for trait ambiguity
///
/// These need to be public for type inference
#[doc(hidden)]
pub struct WasAView;
#[doc(hidden)]
/// See [`WasAView`]
pub struct WasASequence;

impl<AppState, Action, View: MasonryView<AppState, Action>> ViewSequence<AppState, Action, WasAView>
    for View
{
    type State = (<View as MasonryView<AppState, Action>>::State, ViewId);
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::State {
        let (id, state, element) = self.build(cx);
        elements.push(element.boxed());
        (state, id)
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        (view_state, view_id): &mut Self::State,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        let mut element = elements.mutate();
        let downcast = element.downcast::<View::Element>();

        if let Some(element) = downcast {
            self.rebuild(cx, prev, view_id, view_state, element)
        } else {
            unreachable!("Tree structure tracking got wrong element type")
        }
    }

    fn message(
        &self,
        id_path: &[ViewId],
        (view_state, view_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut AppState,
    ) -> MessageResult<Action> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == view_id {
                return self.message(rest_path, view_state, message, app_state);
            }
        }
        MessageResult::Stale(message)
    }

    fn count(&self, _state: &Self::State) -> usize {
        1
    }
}

impl<AppState, Action, Marker, VT: ViewSequence<AppState, Action, Marker>>
    ViewSequence<AppState, Action, (WasASequence, Marker)> for Option<VT>
{
    type State = Option<VT::State>;

    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::State {
        match self {
            None => None,
            Some(vt) => {
                let state = vt.build(cx, elements);
                Some(state)
            }
        }
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        match (self, &mut *state, prev) {
            (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, elements),
            (None, Some(seq_state), Some(prev)) => {
                let count = prev.count(&seq_state);
                elements.delete(count);
                *state = None;

                ChangeFlags::CHANGED // tree structure
            }
            (Some(this), None, None) => {
                *state = Some(this.build(cx, elements));

                ChangeFlags::CHANGED // tree structure
            }
            (None, None, None) => ChangeFlags::UNCHANGED,
            _ => panic!("non matching state and prev value"),
        }
    }

    fn message(
        &self,
        id_path: &[ViewId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut AppState,
    ) -> MessageResult<Action> {
        match (self, state) {
            (Some(vt), Some(state)) => vt.message(id_path, state, message, app_state),
            (None, None) => MessageResult::Stale(message),
            _ => panic!("non matching state and prev value"),
        }
    }

    fn count(&self, state: &Self::State) -> usize {
        match (self, state) {
            (Some(vt), Some(state)) => vt.count(state),
            (None, None) => 0,
            _ => panic!("non matching state and prev value"),
        }
    }
}

// TODO: We use raw indexing for this value. What would make it invalid?
impl<T, A, Marker, VT: ViewSequence<T, A, Marker>> ViewSequence<T, A, (WasASequence, Marker)>
    for Vec<VT>
{
    type State = Vec<VT::State>;
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::State {
        self.iter().map(|child| child.build(cx, elements)).collect()
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::UNCHANGED;
        for ((child, child_prev), child_state) in self.iter().zip(prev).zip(state.iter_mut()) {
            let el_changed = child.rebuild(cx, child_prev, child_state, elements);
            changed.changed |= el_changed.changed;
        }
        let n = self.len();
        if n < prev.len() {
            let n_delete = state
                .splice(n.., [])
                .enumerate()
                .map(|(i, state)| prev[n + i].count(&state))
                .sum();
            elements.delete(n_delete);
            changed.changed |= ChangeFlags::CHANGED.changed; // Tree structure
        } else if n > prev.len() {
            for i in prev.len()..n {
                state.push(self[i].build(cx, elements));
            }
            changed.changed |= ChangeFlags::CHANGED.changed; // Tree structure
        }
        changed
    }

    fn message(
        &self,
        id_path: &[ViewId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let mut result = MessageResult::Stale(message);
        for (child, child_state) in self.iter().zip(state) {
            if let MessageResult::Stale(message) = result {
                result = child.message(id_path, child_state, message, app_state);
            } else {
                break;
            }
        }
        result
    }

    fn count(&self, state: &Self::State) -> usize {
        self.iter()
            .zip(state)
            .map(|(child, child_state)| child.count(child_state))
            .sum()
    }
}

macro_rules! impl_view_tuple {
    (
        // We could use the ${index} metavariable here once it's stable
        // https://veykril.github.io/tlborm/decl-macros/minutiae/metavar-expr.html
        $($marker: ident, $seq: ident, $idx: tt);*
    ) => {
        impl<
                AppState,
                Action,
                $(
                    $marker,
                    $seq: ViewSequence<AppState, Action, $marker>,
                )*
            > ViewSequence<AppState, Action, ($($marker,)*)> for ($($seq,)*)
        {
            type State = ( $( $seq::State, )*);

            fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) -> Self::State {
                let b = ( $( self.$idx.build(cx, elements), )* );
                let state = ( $( b.$idx, )*);
                state
            }

            fn rebuild(
                &self,
                cx: &mut ViewCx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut dyn ElementSplice,
            ) -> ChangeFlags {
                let mut changed = ChangeFlags::UNCHANGED;
                $(
                    let el_changed = self.$idx.rebuild(cx, &prev.$idx, &mut state.$idx, els);
                    changed.changed |= el_changed.changed;
                )*
                changed
            }

            fn message(
                &self,
                id_path: &[ViewId],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut AppState,
            ) -> MessageResult<Action> {
                MessageResult::Stale(message)
                $(
                    .or(|message|{
                        self.$idx.message(id_path, &mut state.$idx, message, app_state)
                    })
                )*
            }

            fn count(&self, state: &Self::State) -> usize {
                0
                $(
                    + self.$idx.count(&state.$idx)
                )*
            }
        }
    }
}

// We implement for tuples of length up to 15.
impl_view_tuple!();
impl_view_tuple!(M0, Seq0, 0);
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
