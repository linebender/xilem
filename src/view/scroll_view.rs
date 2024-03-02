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

use std::{any::Any, marker::PhantomData};

use crate::{view::View, widget::ChangeFlags, MessageResult};

use xilem_core::Id;

use super::{Cx, ViewMarker, ViewSequence};

pub struct ScrollView<T, A, C> {
    child: C,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub fn scroll_view<T, A, C>(child: C) -> ScrollView<T, A, C> {
    ScrollView::new(child)
}

impl<T, A, C> ScrollView<T, A, C> {
    pub fn new(child: C) -> Self {
        ScrollView {
            child,
            phantom: Default::default(),
        }
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewMarker for ScrollView<T, A, VT> {}

impl<T, A, C: View<T, A>> View<T, A> for ScrollView<T, A, C>
where
    C::Element: 'static,
{
    type State = C::State;

    type Element = crate::widget::ScrollView;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, child_element) = cx.with_new_child_pod(|cx| self.child.build(cx));
        let element = crate::widget::ScrollView::new(child_element);
        (id, child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_pod(element.child_mut(), |el, cx| {
            self.child.rebuild(cx, &prev.child, id, state, el)
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.child.message(id_path, state, message, app_state)
    }
}
