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

use crate::view::{Cx, ViewSequence};
use crate::widget::{ChangeFlags, Pod};
use crate::{Id, MessageResult, VecSplice};
use std::any::Any;
use std::marker::PhantomData;

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
pub fn list<T, A, VT: ViewSequence<T, A>, F: Fn(usize) -> VT + Send>(
    items: usize,
    build: F,
) -> List<T, A, VT, F> {
    List {
        items,
        build,
        phantom: PhantomData,
    }
}

impl<T, A, VT: ViewSequence<T, A>, F: Fn(usize) -> VT + Send> ViewSequence<T, A>
    for List<T, A, VT, F>
{
    type State = ListState<T, A, VT>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
        let leading = elements.len();

        let views = (0..self.items)
            .into_iter()
            .map(|index| (self.build)(index))
            .fold(vec![], |mut state, vt| {
                let vt_state = vt.build(cx, elements);
                state.push((vt, vt_state));
                state
            });

        ListState {
            views,
            element_count: elements.len() - leading
        }
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<Pod>,
    ) -> ChangeFlags {
        // Common length
        let leading = element.len();

        let mut flags = (0..(self.items.min(prev.items)))
            .into_iter()
            .zip(&mut state.views)
            .fold(
                ChangeFlags::empty(),
                |flags, (index, (prev, state))| {
                    let vt = (self.build)(index);
                    let vt_flags = vt.rebuild(cx, prev, state, element);
                    *prev = vt;
                    flags | vt_flags
                },
            );

        if self.items < prev.items {
            for (prev, state) in state.views.splice(self.items.., []) {
                element.delete(prev.count(&state));
            }
        }

        while self.items > state.views.len() {
            let vt = (self.build)(state.views.len());
            let vt_state = element.as_vec(|vec|vt.build(cx, vec));
            state.views.push((vt, vt_state));
        }

        // We only check if our length changes. If one of the sub sequences changes thier size they
        // have to set ChangeFlags::all() them self's.
        if self.items != prev.items {
            flags |= ChangeFlags::all();
        }

        state.element_count = element.len() - leading;

        flags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        state
            .views
            .iter_mut()
            .fold(MessageResult::Stale(message), |result, (vt, vt_state)| {
                result.or(|message| vt.message(id_path, vt_state, message, app_state))
            })
    }

    fn count(&self, state: &Self::State) -> usize {
        state.element_count
    }
}
