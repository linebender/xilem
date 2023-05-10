// Copyright 2023 The Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, marker::PhantomData};

use crate::event::MessageResult;
use crate::geometry::Axis;
use crate::id::Id;
use crate::view::ViewMarker;
use crate::widget::{self, ChangeFlags, Pod, Widget};
use crate::VecSplice;

use super::{sizeable::Sizeable, Cx, View};

pub use crate::widget::flex_layout::{CrossAxisAlignment, MainAxisAlignment};

// pub enum FlexChild<V, A, VT: ViewSequence<V, A>> {
//     Fixed {
//         view: VT,
//         alignment: Option<CrossAxisAlignment>,
//         phantom: PhantomData<(V, A)>,
//     },
//     Flex {
//         view: VT,
//         alignment: Option<CrossAxisAlignment>,
//         flex: f64,
//         phantom: PhantomData<(V, A)>,
//     },
// }

// pub trait AsFlexItem<T, A>: Send {
//     type State: Send;
//     type View: View<T, A> + Send;

//     fn item(&self) -> Either<&Self::View, Spacer>;
//     fn alignment(&self) -> Option<CrossAxisAlignment>;
//     fn flex(&self) -> Option<f64>;
// }

pub fn fixed<T, A, V: View<T, A> + Send>(view: V) -> FlexItem<T, A, V> {
    FlexItem {
        view,
        alignment: None,
        flex: None,
        phantom: PhantomData,
    }
}

pub fn flex<T, A, V: View<T, A> + Send>(view: V, flex: f64) -> FlexItem<T, A, V> {
    FlexItem {
        view,
        alignment: None,
        flex: Some(flex),
        phantom: PhantomData,
    }
}

pub fn spacer<T: Send, A: Send>(space: f64) -> FlexItem<T, A, Sizeable<T, A, ()>> {
    FlexItem {
        view: Sizeable::empty().width(space).height(space),
        alignment: None,
        flex: None,
        phantom: PhantomData,
    }
}

pub fn flex_spacer<T: Send, A: Send>(flex: f64) -> FlexItem<T, A, Sizeable<T, A, ()>> {
    FlexItem {
        view: Sizeable::empty().expand(),
        alignment: None,
        flex: Some(flex),
        phantom: PhantomData,
    }
}

pub struct FlexItem<T, A, V: View<T, A> + Send> {
    view: V,
    alignment: Option<CrossAxisAlignment>,
    flex: Option<f64>,
    phantom: PhantomData<(T, A)>,
}

impl<T, A, V: View<T, A> + Send> FlexItem<T, A, V>
where
    V::Element: 'static,
{
    pub fn align(mut self, alignment: CrossAxisAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    // fn build(&self, cx: &mut Cx) -> ((V::State, Id), widget::flex_layout::Child) {
    //     let (id, state, widget) = self.view.build(cx);
    //     let child = widget::flex_layout::Child {
    //         widget: Pod::new(widget),
    //         alignment: self.alignment,
    //         flex: self.flex,
    //     };
    //     ((state, id), child)
    // }

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx, children: &mut Vec<widget::flex_layout::Child>) -> (V::State, Id) {
        let (id, state, widget) = self.view.build(cx);
        let child = widget::flex_layout::Child {
            widget: Pod::new(widget),
            alignment: self.alignment,
            flex: self.flex,
        };
        children.push(child);
        (state, id)
    }

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state_id: &mut (V::State, Id),
        element: &mut VecSplice<widget::flex_layout::Child>,
    ) -> ChangeFlags {
        let (state, id) = state_id;
        let el = &mut element.mutate().widget;
        let downcast = el.downcast_mut().unwrap();
        let flags = self.view.rebuild(cx, &prev.view, id, state, downcast);
        el.mark(flags)
    }

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        id_path: &[Id],
        state_id: &mut (V::State, Id),
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let (state, id) = state_id;
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == id {
                return self.view.message(rest_path, state, message, app_state);
            }
        }
        MessageResult::Stale(message)
    }
}

// #[derive(Debug, Clone, Copy)]
// enum Spacer {
//     Flex(f64),
//     Fixed(f64),
// }

// impl<V, A, VT: View<V, A> + ViewMarker> AsFlexItem<V, A> for VT {
//     type State = (VT::State, Id);
//     type View = VT;

//     fn item(&self) -> Either<&Self::View, Spacer> {
//         Left(self)
//     }

//     fn alignment(&self) -> Option<CrossAxisAlignment> {
//         None
//     }

//     fn flex(&self) -> Option<f64> {
//         None
//     }
// }

// impl<V: Send, A: Send, VT: View<V, A> + ViewMarker + Send> AsFlexItem<V, A> for FlexItem<V, A, VT> {
//     type State = (VT::State, Id);
//     type View = VT;

//     fn item(&self) -> Either<&Self::View, Spacer> {
//         Left(&self.view)
//     }

//     fn alignment(&self) -> Option<CrossAxisAlignment> {
//         self.alignment
//     }

//     fn flex(&self) -> Option<f64> {
//         Some(self.flex)
//     }
// }

// impl<V, A> AsFlexItem<V, A> for Spacer {
//     type State = ();
//     type View = Spacer;

//     fn item(&self) -> Either<&Self::View, Spacer> {
//         Right(*self)
//     }

//     fn alignment(&self) -> Option<CrossAxisAlignment> {
//         None
//     }

//     fn flex(&self) -> Option<f64> {
//         if let Self::Flex(flex) = self {
//             Some(*flex)
//         } else {
//             None
//         }
//     }
// }

/// A sequence on flex items.
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
pub trait FlexItemSequence<T, A>: Send {
    /// Associated states for the views.
    type State: Send;

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx, elements: &mut Vec<widget::flex_layout::Child>) -> Self::State;

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<widget::flex_layout::Child>,
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

macro_rules! replace_tt {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! impl_flex_tuple {
    ( $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T: Send, A: Send, $( $t: View<T, A> + Send ),* > FlexItemSequence<T, A>
            for ( $( FlexItem<T, A, $t>, )* )
            where  $( $t::Element: Widget + 'static ),*
        {
            type State = ( $( ($t::State, Id), )*);

            fn build(&self, cx: &mut Cx, children: &mut Vec<widget::flex_layout::Child>) -> Self::State {
                ( $( self.$i.build(cx, children) ,)* )
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut VecSplice<widget::flex_layout::Child>,
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

            fn count(&self, _state: &Self::State) -> usize {
                0usize
                $(
                    + replace_tt!($i 1usize)
                )*
            }
        }
    }
}

impl_flex_tuple!(V0; 0);
impl_flex_tuple!(V0, V1; 0, 1);
impl_flex_tuple!(V0, V1, V2; 0, 1, 2);
impl_flex_tuple!(V0, V1, V2, V3; 0, 1, 2, 3);
impl_flex_tuple!(V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
impl_flex_tuple!(V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
impl_flex_tuple!(V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
impl_flex_tuple!(V0, V1, V2, V3, V4, V5, V6, V7;
    0, 1, 2, 3, 4, 5, 6, 7
);
impl_flex_tuple!(V0, V1, V2, V3, V4, V5, V6, V7, V8;
    0, 1, 2, 3, 4, 5, 6, 7, 8
);
impl_flex_tuple!(V0, V1, V2, V3, V4, V5, V6, V7, V8, V9;
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9
);

/// FlexLayout is a simple view which does layout for the specified ViewSequence.
///
/// Each Element is positioned on the specified Axis starting at the beginning with the given spacing
///
/// This View is only temporary is probably going to be replaced by something like Druid's Flex
/// widget.
pub struct FlexLayout<T, A, VT: FlexItemSequence<T, A>> {
    children: VT,
    axis: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    phantom: PhantomData<fn() -> (T, A)>,
}

/// creates a vertical [`FlexLayout`].
pub fn v_flex<T, A, VT: FlexItemSequence<T, A>>(children: VT) -> FlexLayout<T, A, VT> {
    FlexLayout::new(children, Axis::Vertical)
}

/// creates a horizontal [`FlexLayout`].
pub fn h_flex<T, A, VT: FlexItemSequence<T, A>>(children: VT) -> FlexLayout<T, A, VT> {
    FlexLayout::new(children, Axis::Horizontal)
}

impl<T, A, VT: FlexItemSequence<T, A>> FlexLayout<T, A, VT> {
    fn new(children: VT, axis: Axis) -> Self {
        let phantom = Default::default();
        FlexLayout {
            children,
            axis,
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,

            phantom,
        }
    }

    /// Builder-style method for specifying the childrens' [`CrossAxisAlignment`].
    ///
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    /// Builder-style method for specifying the childrens' [`MainAxisAlignment`].
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    /// Builder-style method for setting whether the container must expand
    /// to fill the available space on its main axis.
    ///
    /// If any children have flex then this container will expand to fill all
    /// available space on its main axis; But if no children are flex,
    /// this flag determines whether or not the container should shrink to fit,
    /// or must expand to fill.
    ///
    /// If it expands, and there is extra space left over, that space is
    /// distributed in accordance with the [`MainAxisAlignment`].
    ///
    /// The default value is `false`.
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
        self.fill_major_axis = fill;
        self
    }
}

impl<T, A, VT: FlexItemSequence<T, A>> ViewMarker for FlexLayout<T, A, VT> {}

impl<T, A, VT: FlexItemSequence<T, A>> View<T, A> for FlexLayout<T, A, VT> {
    type State = VT::State;

    type Element = widget::flex_layout::FlexLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let Self {
            ref children,
            axis,
            cross_alignment,
            main_alignment,
            fill_major_axis,
            phantom: _phantom,
        } = *self;
        let mut elements = vec![];
        let (id, state) = cx.with_new_id(|cx| children.build(cx, &mut elements));
        let column = widget::flex_layout::FlexLayout::new(
            elements,
            axis,
            cross_alignment,
            main_alignment,
            fill_major_axis,
        );
        (id, state, column)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut scratch = vec![];
        let mut splice = VecSplice::new(&mut element.children, &mut scratch);

        let mut flags = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        });

        // if self.spacing != prev.spacing || self.axis != prev.axis {
        if self.axis != prev.axis
            || self.cross_alignment != prev.cross_alignment
            || self.main_alignment != prev.main_alignment
            || self.fill_major_axis != prev.fill_major_axis
        {
            element.axis = self.axis;
            element.cross_alignment = self.cross_alignment;
            element.main_alignment = self.main_alignment;
            element.fill_major_axis = self.fill_major_axis;
            flags |= ChangeFlags::LAYOUT;
        }

        flags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.children.message(id_path, state, event, app_state)
    }
}
