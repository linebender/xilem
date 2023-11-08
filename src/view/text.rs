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

use crate::view::{Id, ViewMarker};
use crate::widget::ChangeFlags;

use super::{Cx, View};

impl ViewMarker for String {}

impl<T, A> View<T, A> for String {
    type State = ();

    type Element = crate::widget::TextWidget;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| crate::widget::TextWidget::new(self.clone()));
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
        if prev != self {
            element.set_text(self.clone())
        } else {
            ChangeFlags::empty()
        }
    }

    fn message(
        &self,
        _id_path: &[xilem_core::Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        xilem_core::MessageResult::Stale(message)
    }
}
