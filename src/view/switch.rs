// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::any::Any;

use crate::app;
use crate::view::ViewMarker;
use crate::{view::Id, widget::ChangeFlags, MessageResult};

use super::{Cx, View};

pub struct Switch<T, A> {
    is_on: bool,
    callback: Box<dyn Fn(&mut T, bool) -> A + Send>,
}

pub fn switch<T, A>(
    is_on: bool,
    clicked: impl Fn(&mut T, bool) -> A + Send + 'static,
) -> Switch<T, A> {
    Switch::new(is_on, clicked)
}

impl<T, A> Switch<T, A> {
    pub fn new(is_on: bool, clicked: impl Fn(&mut T, bool) -> A + Send + 'static) -> Self {
        Switch {
            is_on,
            callback: Box::new(clicked),
        }
    }
}

impl<T, A> ViewMarker for Switch<T, A> {}

impl<T, A> View<T, A> for Switch<T, A> {
    type State = ();

    type Element = crate::widget::Switch;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) =
            cx.with_new_id(|cx| crate::widget::Switch::new(cx.id_path(), self.is_on));
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
        if prev.is_on != self.is_on {
            element.set_is_on(self.is_on)
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
        MessageResult::Action((self.callback)(app_state, !self.is_on))
    }
}
