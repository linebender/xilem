use crate::event::MessageResult;
use crate::id::Id;
use crate::view::{Cx, View, ViewMarker};
use crate::widget::{ChangeFlags, Pod, Widget};
use crate::VecSplice;
use std::any::Any;
use std::marker::PhantomData;
use std::ops::Range;

/// A sequence on view nodes.
///
/// This is one of the central traits for representing UI. Every view which has a collection of
/// children uses an instance of this trait to specify them.
///
/// The framework will then run methods on these views to create the associated
/// state tree and widget tree, as well as incremental updates and event
/// propagation. The methods in the `ViewSequence` trait correspond to the ones in the `View` trait.
///
/// The `View` trait is parameterized by `T`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in event
/// propagation. During event handling, mutable access to the app state is
/// given to view nodes, which in turn can expose it to callbacks.
pub trait ViewSequence<T, A = ()>: Send {
    /// Associated states for the views.
    type State: Send;

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State;

    /// Update the associated widgets.
    ///
    /// Returns the merged change flags of all its loaded children and the new `SequencePosition` of
    /// the element which received focus, by calling `set_focus` on this sequence.
    ///
    /// This value can be used to keep a certain widget inside the viewport after `rebuild`.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<Pod>,
    ) -> (ChangeFlags, SequencePosition);

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;

    /// Returns the current amount of widgets build by this sequence.
    fn count(&self, state: &Self::State) -> usize;

    /// Returns an estimate of the amount of elements this sequence could load.
    fn virtual_count(&self, state:&Self::State) -> usize;

    /// Control which part of this sequence is loaded.
    ///
    /// `focus` marks the element which should be kept as reference.
    /// `range` tells which the range of elements relative to `focus` should be loaded.
    ///
    /// When this method was never called, the sequence should try to load all it's elements.
    ///
    /// When interpreting `focus` the sequence should not factor in changes which happened after
    /// the last `build` or `rebuild`. For an example: If this sequence received an event insert an
    /// element at index 1 a `focus` value of 2 should still point to the old element loaded at
    /// index 2. The conversion happens during rebuild!
    ///
    /// The return value is an estimate of the amount of elements available before and after the
    /// provided focus.
    fn set_focus(&mut self, state: &mut Self::State, focus: SequencePosition, range: Range<isize>) -> Range<isize>;
}

/// The focused element of a view sequence.
///
/// The focused element is the element the Sequence tries to load and keeps track of. This is useful
/// if you want to keep a certain element focused even after rebuilding the sequence and doing
/// layout.
pub enum SequencePosition {
    /// A value between 0 and 1, marking the approximate position of an element in this sequence.
    ///
    /// This variant can be used to index into the sequence before any elements are loaded, or for
    /// jumping to a certain position without loading all elements between. 0 is the first element
    /// and 1 the last one.
    Fraction(f64),
    /// An offset to a specific element in the sequence.
    ///
    /// The index of this variant is not constrained to to `0..seq.len()`. If the value lies
    /// outside these bounds, the sequence should load the element.
    ///
    /// if deleted is set the element this position points to no longer exists.
    ///
    /// The focus of a sequence may change after rebuild!
    Index {
        index: isize,
        deleted: bool,
    },

}

impl SequencePosition {
    fn index(index: isize) -> Self {
        Self::Index {index, deleted: false}
    }

    fn deleted(index: isize) -> Self {
        Self::Index {index, deleted: true}
    }

    fn fraction(fraction: f64) -> Self {
        Self::Fraction(fraction)
    }
}

// ViewMarker is already a dependency of View but Rusts orphan rules dont work if we remove it here.
impl<T, A, V: View<T, A> + ViewMarker> ViewSequence<T, A> for V
where
    V::Element: Widget + 'static,
{
    type State = (<V as View<T, A>>::State, Id);

    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
        let (id, state, element) = <V as View<T, A>>::build(self, cx);
        elements.push(Pod::new(element));
        (state, id)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<Pod>,
    ) -> (ChangeFlags, Focus) {
        let el = element.mutate();
        let downcast = el.downcast_mut().unwrap();
        let flags =
            <V as View<T, A>>::rebuild(self, cx, prev, &mut state.1, &mut state.0, downcast);

        (el.mark(flags), SequencePosition::index(0))
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == &state.1 {
                return <V as View<T, A>>::message(
                    self,
                    rest_path,
                    &mut state.0,
                    message,
                    app_state,
                );
            }
        }
        MessageResult::Stale(message)
    }

    fn count(&self, _state: &Self::State) -> usize {
        1
    }

    fn virtual_count(&self, state: &Self::State) -> usize {
        1
    }

    fn set_focus(&mut self, _state: &mut Self::State, _focus: SequencePosition, _range: Range<isize>) -> Range<isize> {
        0..1
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewSequence<T, A> for Option<VT> {
    type State = Option<VT::State>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
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
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<Pod>,
    ) -> (ChangeFlags, SequencePosition) {
        match (self, &mut *state, prev) {
            (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, element),
            (None, Some(seq_state), Some(prev)) => {
                let count = prev.count(&seq_state);
                element.delete(count);
                *state = None;

                (ChangeFlags::all(), SequencePosition::deleted(0))
            }
            (Some(this), None, None) => {
                let seq_state = element.as_vec(|vec| this.build(cx, vec));
                *state = Some(seq_state);

                (ChangeFlags::all(), SequencePosition::index(0))
            }
            (None, None, None) => (ChangeFlags::empty(), SequencePosition::deleted(0)),
            _ => panic!("non matching state and prev value"),
        }
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
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

    fn virtual_count(&self, state: &Self::State) -> usize {
        match (self, state) {
            (Some(vt), Some(state)) => vt.virtual_count(state),
            (None, None) => 0,
            _ => panic!("non matching state and prev value"),
        }
    }

    fn set_focus(&mut self, state: &mut Self::State, focus: SequencePosition, range: Range<isize>) -> Range<isize> {
        match (self, state) {
            (Some(vt), Some(state)) => vt.set_focus(state, focus, range),
            (None, None) => 0..0,
            _ => panic!("non matching state and prev value"),
        }
    }
}

struct SeqInfo {
    count: usize,
    virtual_count: usize,
}

fn child_to_parent_position(child_position: SequencePosition, child_index:usize, elements: &[SeqInfo]) -> SequencePosition {

}

fn parent_to_child_position(parent_position: SequencePosition, elements: &[SeqInfo]) -> (SequencePosition, usize) {

}

macro_rules! impl_view_tuple {
    ( $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: ViewSequence<T, A> ),* > ViewSequence<T, A> for ( $( $t, )* ) {
            type State = ( ($( $t::State, )*), [SeqInfo; ], usize);

            fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
                let b = ( $( self.$i.build(cx, elements), )* );
                let info = [$({
                    let count = self.$i.count(&b.$i);
                    let virtual_count = self.$i.virtual_count(&b.$i);
                    ChildInfo {
                        count,
                        virtual_count,
                    }
                },)*];

                (
                    ($( b.$i, )*),
                    info,
                    0,
                )
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut VecSplice<Pod>,
            ) -> (ChangeFlags, SequencePosition) {
                let mut changed = ChangeFlags::default();
                let mut position = SequencePosition::deleted(0);

                $(
                    let (el_changed, el_index) = self.$i.rebuild(cx, &prev.$i, &mut state.0.$i, els);
                    if *state.1 == $i {
                        position = el_index;
                    }
                    state.1[$i].count = self.$i.count(&state.0.$i);
                    state.1[$i].virtual_count = self.$i.virtual_count(&state.0.$i);
                    changed |= el_changed;
                )*

                let position = child_to_parent_position(position, state.2, &state.1);

                (changed, position)
            }

            fn message(
                &self,
                id_path: &[Id],
                state: &mut Self::State,
                message: Box<dyn Any>,
                app_state: &mut T,
            ) -> MessageResult<A> {
                MessageResult::Stale(message)
                $(
                    .or(|message|{
                        self.$i.message(id_path, &mut state.0.$i, message, app_state)
                    })
                )*
            }

            fn count(&self, state: &Self::State) -> usize {
                state.1.count
            }

            fn virtual_count(&self, state: &Self::State) -> usize {
                state.1.virtual_count
            }

            fn set_focus(&mut self, state: &mut Self::State, focus: SequencePosition, range: Range<isize>) -> Range<isize> {
                let (position, index) = parent_to_child_position(focus, &state.1);
                state.2 = index;
                let mut rem = 0..0;

                $(
                    if $i == index {
                        rem = self.$i.set_focus(position, range);
                    }
                )*
                let prev = (range.start - rem.start).max(0);
                let after = (range.end - rem.end).max(0);

                $(
                    if $i < index {
                        let req = prev - state.1[($i+1)..index].fold(0, |i, c|i + c.virtual_count)
                        if req > 0 {
                            self.$i.set_focus(state.0.$i, SequencePosition::fraction(1.0), -req..0);
                        }
                    }
                    if $i > index {
                        let req = after - state.1[index..($i-1)].fold(0, |i, c|i + c.virtual_count)
                        if req > 0 {
                            self.$i.set_focus(state.0.$i, SequencePosition::fraction(0.0), 0..req);
                        }
                    }
                )*
            }
        }
    }
}

impl_view_tuple!(V0; 0);
impl_view_tuple!(V0, V1; 0, 1);
impl_view_tuple!(V0, V1, V2; 0, 1, 2);
impl_view_tuple!(V0, V1, V2, V3; 0, 1, 2, 3);
impl_view_tuple!(V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
impl_view_tuple!(V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
impl_view_tuple!(V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
impl_view_tuple!(V0, V1, V2, V3, V4, V5, V6, V7;
    0, 1, 2, 3, 4, 5, 6, 7
);
impl_view_tuple!(V0, V1, V2, V3, V4, V5, V6, V7, V8;
    0, 1, 2, 3, 4, 5, 6, 7, 8
);
impl_view_tuple!(V0, V1, V2, V3, V4, V5, V6, V7, V8, V9;
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9
);
