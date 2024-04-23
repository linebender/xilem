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

use crate::view::{Cx, Id, View, ViewMarker};
use crate::{widget::ChangeFlags, MessageResult};

use masonry::widget::WidgetMut;
use masonry::WidgetId;

pub struct Switch<T, A> {
    is_on: bool,
    #[allow(clippy::type_complexity)]
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

    type Element = masonry::widget::Checkbox;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| masonry::widget::Checkbox::new(self.is_on, ""));
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        if prev.is_on != self.is_on {
            element.set_checked(self.is_on)
        }
        ChangeFlags::default()
    }

    fn message(
        &self,
        _id_path: &[WidgetId],
        _state: &mut Self::State,
        _message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Action((self.callback)(app_state, !self.is_on))
    }
}
