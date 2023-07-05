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

use xilem_core::{Id, MessageResult};

use crate::widget::{ChangeFlags, TextWidget};

use super::{Cx, View, ViewMarker};

impl ViewMarker for String {}
impl<T, A> View<T, A> for String {
    type State = ();

    type Element = TextWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| TextWidget::new(self.clone()));
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
        let mut change_flags = ChangeFlags::empty();
        if prev != self {
            change_flags |= element.set_text(self.clone());
        }
        change_flags
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

impl ViewMarker for &'static str {}
impl<T, A> View<T, A> for &'static str {
    type State = ();

    type Element = TextWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| TextWidget::new(self.to_string()));
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
        let mut change_flags = ChangeFlags::empty();
        if prev != self {
            change_flags |= element.set_text(self.to_string());
        }
        change_flags
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}
