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
use crate::{Id, MessageResult};
use crate::view::{Cx, ViewSequence};
use crate::widget::{ChangeFlags, Pod};

/// A simple view sequence which builds a dynamic amount of sub sequences.
pub struct List<T, A, VT: ViewSequence<T, A>, F: Fn(usize) -> VT + Send> {
    items: usize,
    build: F,
    phantom: PhantomData<fn() -> (T, A, VT)>,
}

/// The state of a List sequence
pub struct ListState<T, A, VT: ViewSequence<T, A>> {
    views: Vec<(VT, VT::State)>,
    element_count: usize,
}

/// creates a new `List` sequence.
pub fn list<T, A, VT: ViewSequence<T, A>, F: Fn(usize) -> VT + Send>(items: usize, build: F) -> List<T, A, VT, F> {
    List {
        items,
        build,
        phantom: PhantomData,
    }
}

impl<T, A, VT: ViewSequence<T, A>, F: Fn(usize) -> VT + Send> ViewSequence<T, A> for List<T, A, VT, F> {
    type State = ListState<T, A, VT>;

    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<Pod>) {
        let (views, elements) = (0..self.items).into_iter()
            .map(|index|(self.build)(index))
            .fold((vec![], vec![]), |(mut state, mut elements), vt|{
                let (vt_state, mut vt_elements) = vt.build(cx);
                state.push((vt, vt_state));
                elements.append(&mut vt_elements);
                (state, elements)
            });

        let element_count = elements.len();
        (ListState {views, element_count}, elements)
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, offset: usize, element: &mut Vec<Pod>) -> (ChangeFlags, usize) {
        let prev_elements = element.len();
        // Common length
        let (mut flags, mut new_offset) = (0..(self.items.min(prev.items))).into_iter()
            .zip(&mut state.views)
            .fold((ChangeFlags::empty(), offset), |(flags, offset), (index, (prev, state))|{
                let vt = (self.build)(index);
                let (vt_flags, new_offset) = vt.rebuild(cx, prev, state, offset, element);
                *prev = vt;
                (flags | vt_flags, new_offset)
            });

        while element.len() > state.element_count {
            // If this list shrinks, it removes the always the last views.
            // offset is the first element after the rebuild elements
            element.remove(new_offset);
        }

        while state.views.len() > self.items {
            state.views.pop();
        }

        while self.items > state.views.len() {
            let vt = (self.build)(state.views.len());
            let (vt_state, elements) = vt.build(cx);
            state.views.push((vt, vt_state));
            let count = elements.len();
            new_offset = elements.into_iter().fold(new_offset, |new_offset, pod|{element.insert(new_offset, pod); new_offset + 1});
        }

        state.element_count = new_offset - offset;

        // We only check if our length changes. If one of the sub sequences changes thier size they
        // have to set ChangeFlags::all() them self's.
        if self.items != prev.items {
            flags |= ChangeFlags::all();
        }

        (flags, new_offset)
    }

    fn message(&self, id_path: &[Id], state: &mut Self::State, message: Box<dyn Any>, app_state: &mut T) -> MessageResult<A> {
        state.views.iter_mut()
            .fold(MessageResult::Stale(message), |result, (vt, vt_state)|{ result.or(|message|{
                vt.message(id_path, vt_state, message, app_state)
            })})
    }

    fn count(&self, state: &Self::State) -> usize {
        state.element_count
    }
}