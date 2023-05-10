// Copyright 2023 The Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, f64::INFINITY, marker::PhantomData};

use crate::{
    event::MessageResult,
    id::Id,
    widget::{self, ChangeFlags, Pod},
};

use super::{Cx, View, ViewMarker};

pub struct Sizeable<T, A, V: View<T, A> + Send> {
    child: Option<V>,
    width: Option<f64>,
    height: Option<f64>,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A> Sizeable<T, A, ()> {
    pub fn empty() -> Self {
        Sizeable {
            child: None,
            width: None,
            height: None,
            phantom: PhantomData,
        }
    }
}

impl<T, A, V: View<T, A> + Send> Sizeable<T, A, V> {
    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(INFINITY);
        self.height = Some(INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(INFINITY);
        self
    }
}

pub fn sizeable<T, A, V: View<T, A> + Send>(view: V) -> Sizeable<T, A, V> {
    Sizeable {
        child: Some(view),
        width: None,
        height: None,
        phantom: PhantomData,
    }
}

impl ViewMarker for () {}

impl<T, A> View<T, A> for () {
    type State = ();

    type Element = ();

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        (cx.with_new_id(|_| ()).0, (), ())
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        _element: &mut Self::Element,
    ) -> ChangeFlags {
        ChangeFlags::empty()
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

impl<T, A, V: View<T, A> + Send> ViewMarker for Sizeable<T, A, V> {}

impl<T: Send, A: Send, V: View<T, A> + Send> View<T, A> for Sizeable<T, A, V>
where
    V::Element: 'static,
{
    type State = Option<(V::State, Id)>;

    type Element = widget::Sizeable;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (state, child) = self
                .child
                .as_ref()
                .map(|child| {
                    let (inner_id, state, widget) = child.build(cx);
                    ((state, inner_id), Pod::new(widget))
                })
                .unzip();

            let element = widget::Sizeable {
                child,
                width: self.width,
                height: self.height,
                old_size: None,
            };

            (state, element)
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut crate::id::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut flags = if let Some((self_child, (state, inner_id))) =
            self.child.as_ref().zip(state.as_mut())
        {
            let pod = &mut element.child.as_mut().unwrap();
            let element = pod.downcast_mut();
            cx.with_id(*id, |cx| {
                self_child.rebuild(
                    cx,
                    prev.child.as_ref().unwrap(),
                    inner_id,
                    state,
                    element.unwrap(),
                )
            })
        } else {
            ChangeFlags::empty()
        };
        if self.width != prev.width || self.height != prev.height {
            element.width = self.width;
            element.height = self.height;
            flags |= ChangeFlags::LAYOUT;
        }
        flags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        // state
        //     .as_mut()
        //     .zip(self.child.as_ref())
        //     .zip(id_path.split_first())
        //     .and_then(|(((state, id), child), (first, rest_path))| {
        //         (first == id).then(|| child.message(rest_path, state, message, app_state))
        //     })
        //     .unwrap_or(MessageResult::Nop)
        if let Some((state, id)) = state {
            id_path
                .split_first()
                .map_or(MessageResult::Nop, |(first, rest_path)| {
                    if first == id {
                        self.child
                            .as_ref()
                            .unwrap()
                            .message(rest_path, state, message, app_state)
                    } else {
                        MessageResult::Nop
                    }
                })
        } else {
            MessageResult::Nop
        }
    }
}
