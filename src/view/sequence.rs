use crate::event::MessageResult;
use crate::id::Id;
use crate::view::{Cx, View};
use crate::widget::{ChangeFlags, Pod};
use std::any::Any;

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
        element: &mut Vec<Pod>,
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
}

macro_rules! impl_view_tuple {
    ( $n: tt; $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, A, $( $t: View<T, A> ),* > ViewSequence<T, A> for ( $( $t, )* )
            where $( <$t as View<T, A>>::Element: 'static ),*
        {
            type State = ( $( $t::State, )* [Id; $n]);

            fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>) {
                let b = ( $( self.$i.build(cx), )* );
                let state = ( $( b.$i.1, )* [ $( b.$i.0 ),* ]);
                let els = vec![ $( Pod::new(b.$i.2) ),* ];
                (state, els)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut Vec<Pod>,
            ) -> ChangeFlags {
                let mut changed = ChangeFlags::default();
                $({
                    let el_changed = self.$i.rebuild(cx, &prev.$i, &mut state.$n[$i], &mut state.$i, els[$i].downcast_mut().unwrap());
                    changed |= els[$i].mark(el_changed);
                })*

                changed
            }

            fn message(
                &self,
                id_path: &[Id],
                state: &mut Self::State,
                message: Box<dyn Any>,
                app_state: &mut T,
            ) -> MessageResult<A> {
                let hd = id_path[0];
                let tl = &id_path[1..];
                $(
                if hd == state.$n[$i] {
                    self.$i.message(tl, &mut state.$i, message, app_state)
                } else )* {
                    crate::event::MessageResult::Stale
                }
            }
        }
    }
}

impl_view_tuple!(1; V0; 0);
impl_view_tuple!(2; V0, V1; 0, 1);
impl_view_tuple!(3; V0, V1, V2; 0, 1, 2);
impl_view_tuple!(4; V0, V1, V2, V3; 0, 1, 2, 3);
impl_view_tuple!(5; V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
impl_view_tuple!(6; V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
impl_view_tuple!(7; V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
impl_view_tuple!(8;
    V0, V1, V2, V3, V4, V5, V6, V7;
    0, 1, 2, 3, 4, 5, 6, 7
);
impl_view_tuple!(9;
    V0, V1, V2, V3, V4, V5, V6, V7, V8;
    0, 1, 2, 3, 4, 5, 6, 7, 8
);
impl_view_tuple!(10;
    V0, V1, V2, V3, V4, V5, V6, V7, V8, V9;
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9
);
