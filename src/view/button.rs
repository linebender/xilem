// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use crate::view::{Id, ViewMarker};
use crate::widget::ChangeFlags;
use crate::MessageResult;

use super::{Cx, View};

pub struct Button<T, A> {
    label: String,
    // consider not boxing
    callback: Box<dyn Fn(&mut T) -> A + Send>,
}

pub fn button<T, A>(
    label: impl Into<String>,
    clicked: impl Fn(&mut T) -> A + Send + 'static,
) -> Button<T, A> {
    Button::new(label, clicked)
}

impl<T, A> Button<T, A> {
    pub fn new(label: impl Into<String>, clicked: impl Fn(&mut T) -> A + Send + 'static) -> Self {
        Button {
            label: label.into(),
            callback: Box::new(clicked),
        }
    }
}

impl<T, A> ViewMarker for Button<T, A> {}

impl<T, A> View<T, A> for Button<T, A> {
    type State = ();

    type Element = crate::widget::Button;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) =
            cx.with_new_id(|cx| crate::widget::Button::new(cx.id_path(), self.label.clone()));
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        if prev.label != self.label {
            element.set_label(self.label.clone())
        } else {
            ChangeFlags::default()
        }
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Action((self.callback)(app_state))
    }
}
