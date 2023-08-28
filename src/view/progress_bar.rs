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
use std::marker::PhantomData;

use crate::view::ViewMarker;
use crate::{view::Id, widget::ChangeFlags, MessageResult};

use super::{Cx, View};

pub struct ProgressBar<T> {
    value: f64,
    data: PhantomData<T>,
}

pub fn progress<T>(value: f64) -> ProgressBar<T> {
    ProgressBar::new(value)
}

impl<T> ProgressBar<T> {
    pub fn new(value: f64) -> Self {
        ProgressBar {
            value: value.clamp(0.0, 1.0),
            data: PhantomData,
        }
    }
}

impl<T> ViewMarker for ProgressBar<T> {}

impl<T: Send, A> View<T, A> for ProgressBar<T> {
    type State = ();

    type Element = crate::widget::ProgressBar;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_cx| crate::widget::ProgressBar::new(self.value));
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
        if prev.value != self.value {
            element.set_value(self.value)
        } else {
            ChangeFlags::default()
        }
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
