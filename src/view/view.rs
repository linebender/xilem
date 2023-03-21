use crate::event::MessageResult;
use crate::id::Id;
use crate::view::{Cx, TypedView};
use crate::widget::{ChangeFlags, Pod, Widget};
use std::any::Any;
use crate::{Element, VecSplice};

/// A view object representing a node in the UI.
///
/// This is a central trait for representing UI. An app will generate a tree of
/// these objects (the view tree) as the primary interface for expressing UI.
/// The view tree is transitory and is retained only long enough to dispatch
/// events and then serve as a reference for diffing for the next view tree.
///
/// For simplicity every view builds a list of elements. The interface for this is specified
/// in [`ViewSequence`]. `View` acts as compile time guarantee that the list has length of one.
/// The reason for this implementation is that the set of `View`s is a subset of the set of
/// `ViewSequence`s. Every sequence which knows its length to be one also implements `View`
///
/// This is the trait you should use when composing views. For implementing them take a look at
/// [`TypedView`]. `View` and `ViewSequence` are implemented for every `TypedView`!
///
/// [`TypedView`]: crate::view::TypedView
pub trait View<E: Element, T, A = ()>: ViewSequence<E, T, A> {

}

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
pub trait ViewSequence<E: Element, T, A = ()>: Send {
    /// Associated states for the views.
    type State: Send;

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State;

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<E>,
    ) -> ChangeFlags;

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


impl<E: Element, T, A, VT: ViewSequence<E, T, A>> ViewSequence<E, T, A> for Option<VT> {
    type State = Option<VT::State>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State {
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
        element: &mut VecSplice<E>,
    ) -> ChangeFlags {
        match (self, &mut *state, prev) {
            (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, element),
            (None, Some(seq_state), Some(prev)) => {
                let mut count = prev.count(&seq_state);
                element.delete(count);
                *state = None;

                ChangeFlags::all()
            }
            (Some(this), None, None) => {
                let seq_state = element.as_vec(|vec|this.build(cx, vec));
                *state = Some(seq_state);

                ChangeFlags::all()
            }
            (None, None, None) => ChangeFlags::empty(),
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
}

macro_rules! impl_view_tuple {
    ( $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<E: Element, T, A, $( $t: ViewSequence<E, T, A> ),* > ViewSequence<E, T, A> for ( $( $t, )* ) {
            type State = ( $( $t::State, )*);

            fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State {
                let mut b = ( $( self.$i.build(cx, elements), )* );
                let state = ( $( b.$i, )*);
                state
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut VecSplice<E>,
            ) -> ChangeFlags {
                let mut changed = ChangeFlags::default();
                $(
                    let el_changed = self.$i.rebuild(cx, &prev.$i, &mut state.$i, els);
                    changed |= el_changed;
                )*
                changed
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
