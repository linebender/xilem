use std::any::Any;
use crate::event::MessageResult;
use crate::id::Id;
use crate::{View, Widget};
use crate::view::Cx;
use crate::widget::{ChangeFlags, Pod};

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
/// given to view nodes, which in turn can make expose it to callbacks.
pub trait ViewSequence<T, A = ()>: Send {
    /// Associated states for the views.
    type State: Send;

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        offset: usize,
        element: &mut Vec<Pod>,
    ) -> (ChangeFlags, usize);

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
}

impl<T, A, V: View<T, A>> ViewSequence<T, A> for V where V::Element: Widget + 'static {
    type State = (<V as View<T, A>>::State, Id);

    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>) {
        let (id, state, element) = <V as View<T, A>>::build(self, cx);
        ((state, id), vec![Pod::new(element)])
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, offset: usize, element: &mut Vec<Pod>) -> (ChangeFlags, usize) {
        let downcast = element[offset].downcast_mut().unwrap();
        let flags = <V as View<T, A>>::rebuild(self, cx, prev, &mut state.0, &mut state.1, downcast);
        let flags = element[offset].mark(flags);
        (flags, offset + 1)
    }

    fn message(&self, id_path: &[Id], state: &mut Self::State, message: Box<dyn Any>, app_state: &mut T) -> MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == state.0 {
                return <V as View<T, A>>::message(self, rest_path, &mut state.1, message, app_state);
            }
        }
        MessageResult::Stale(message)
    }

    fn count(&self, state: &Self::State) -> usize {
        1
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewSequence<T, A> for Option<VT> {
    type State = Option<VT::State>;

    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>) {
        match self {
            None => (None, vec![]),
            Some(vt) => {
                let (state, elements) = vt.build();
                (Some(state), elements)
            }
        }
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, offset: usize, element: &mut Vec<Pod>) -> (ChangeFlags, usize) {
        match (self, state, prev) {
            (Some(this), Some(state), Some(prev)) => {
                this.rebuild(cx, prev, state, offset, element)
            }
            (None, Some(state), Some(prev)) => {
                let mut count = prev.count(&state);
                while count > 0 {
                    element.remove(offset);
                }
                *state = None;
                (ChangeFlags::TREE, offset)
            }
            (Some(this), None, None) => {
                let (state, mut elements) = this.build(cx);
                let additional = elements.len();
                *state = Some(state);
                while !elements.is_empty() {
                    element.insert(offset, elements.pop().unwrap());
                }
                (ChangeFlags::TREE, offset + additional)
            }
            (None, None, None) => (ChangeFlags::empty(), offset),
            _ => panic!("non matching state and prev value"),
        }
    }

    fn message(&self, id_path: &[Id], state: &mut Self::State, message: Box<dyn Any>, app_state: &mut T) -> MessageResult<A> {
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

macro_rules! impl_view_tuple {
    ( $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: ViewSequence<T, A> ),* > ViewSequence<T, A> for ( $( $t, )* ) {
            type State = ( $( $t::State, )*);

            fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>) {
                let b = ( $( self.$i.build(cx), )* );
                let state = ( $( b.$i.0, )*);
                let mut els = vec![];
                $( els.append(b.$i.1); )*
                (state, els)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                offset: usize,
                els: &mut Vec<Pod>,
            ) -> (ChangeFlags, usize) {
                let mut changed = ChangeFlags::default();
                $(
                    let (el_changed, offset) = self.$i.rebuild(cx, &prev.$i, &mut state.$i, offset, els);
                    changed |= el_changed;
                )*
                (changed, offset)
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
                        self.$i.message(id_path, &mut state.$i, message, app_state)
                    })
                )*
            }

            fn count(&self, state: &Self::State) -> usize {
                0
                $(
                    + self.$i.count(&state.$i)
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