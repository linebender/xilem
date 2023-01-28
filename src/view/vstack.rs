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

use crate::{event::EventResult, id::Id, view_seq::ViewSequence, widget::WidgetTuple};
use crate::event::MessageResult;
use crate::geometry::Axis;
use crate::view::sequence::ViewSequence;
use crate::widget::ChangeFlags;

use super::{Cx, View};

pub struct VStack<T, A, VT: ViewSequence<T, A>> {
    children: VT,
    spacing: f64,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub fn v_stack<T, A, VT: ViewSequence<T, A>>(children: VT) -> VStack<T, A, VT> {
    VStack::new(children)
}

impl<T, A, VT: ViewSequence<T, A>> VStack<T, A, VT> {
    pub fn new(children: VT) -> Self {
        let phantom = Default::default();
        VStack { children, phantom, spacing: 0.0 }
    }

    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<T, A, VT: ViewSequence<T, A>> View<T, A> for VStack<T, A, VT>
where
    VT::Elements: WidgetTuple,
{
    type State = VT::State;

    type Element = crate::widget::linear_layout::LinearLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, elements)) = cx.with_new_id(|cx| self.children.build(cx));
        let column = crate::widget::linear_layout::LinearLayout::new(elements, self.spacing, Axis::Vertical);
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
        let mut flags = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, element.children_mut())
        });

        if self.spacing != prev.spacing {
            *element.spacing_mut() = self.spacing;
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
