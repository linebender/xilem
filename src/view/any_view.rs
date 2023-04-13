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

use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use crate::view::{View, ViewSequence};
use crate::{Element, event::MessageResult, id::Id, VecSplice, widget::{AnyWidget, ChangeFlags}};

use super::{Cx, TypedView};

pub trait AnyView<E, T, A = ()>: AnySequence<E, T, A> {}

/// A trait enabling type erasure of views.
///
/// The name is slightly misleading as it's not any view, but only ones
/// whose element is AnyWidget.
///
/// Making a trait which is generic over another trait bound appears to
/// be well beyond the capability of Rust's type system. If type-erased
/// views with other bounds are needed, the best approach is probably
/// duplication of the code, probably with a macro.
pub trait AnySequence<E, T, A = ()> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Box<dyn Any + Send>;

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnySequence<E, T, A>,
        state: &mut Box<dyn Any + Send>,
        element: &mut VecSplice<E>,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;

    fn dyn_count(&self, state: &Box<dyn Any + Send>) -> usize;
}

impl<E: Element, T, A, V: View<E, T, A> + 'static> AnyView<E, T, A> for V
where
    <V as ViewSequence<E, T, A>>::State: 'static,
{}

impl<E: Element, T, A, V: ViewSequence<E, T, A> + 'static> AnySequence<E, T, A> for V
where
    V::State: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Box<dyn Any + Send> {
        Box::new(self.build(cx, elements))
    }

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnySequence<E, T, A>,
        state: &mut Box<dyn Any + Send>,
        element: &mut VecSplice<E>,
    ) -> ChangeFlags {
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                self.rebuild(cx, prev, state, element)
            } else {
                println!("downcast of state failed in dyn_rebuild");
                ChangeFlags::default()
            }
        } else {
            let new_state = element.as_vec(|vec|self.build(cx, vec));
            *state = Box::new(new_state);

            // Everything about the new view could be different, so return all the flags
            ChangeFlags::all()
        }
    }

    fn dyn_message(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some(state) = state.downcast_mut() {
            self.message(id_path, state, message, app_state)
        } else {
            // Possibly softer failure?
            panic!("downcast error in dyn_event");
        }
    }

    fn dyn_count(&self, state: &Box<dyn Any + Send>) -> usize {
        self.count(state.downcast_ref().unwrap())
    }
}

impl<E: Element, T: 'static, A: 'static> ViewSequence<E, T, A> for Box<dyn AnySequence<E, T, A> + Send> {
    type State = Box<dyn Any + Send>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State {
        self.deref().dyn_build(cx, elements)
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, element: &mut VecSplice<E>) -> ChangeFlags {
        self.deref().dyn_rebuild(cx, prev, state, element)
    }

    fn message(&self, id_path: &[Id], state: &mut Self::State, message: Box<dyn Any>, app_state: &mut T) -> MessageResult<A> {
        self.deref().dyn_message(id_path, state, message, app_state)
    }

    fn count(&self, state: &Self::State) -> usize {
        self.deref().dyn_count(state)
    }
}



impl<E: Element, T: 'static, A: 'static> View<E, T, A> for Box<dyn AnyView<E, T, A> + Send> {}

impl<E: Element, T: 'static, A: 'static> ViewSequence<E, T, A> for Box<dyn AnyView<E, T, A> + Send> {
    type State = Box<dyn Any + Send>;

    fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State {
        self.deref().dyn_build(cx, elements)
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, element: &mut VecSplice<E>) -> ChangeFlags {
        self.deref().dyn_rebuild(cx, prev, state, element)
    }

    fn message(&self, id_path: &[Id], state: &mut Self::State, message: Box<dyn Any>, app_state: &mut T) -> MessageResult<A> {
        self.deref().dyn_message(id_path, state, message, app_state)
    }

    fn count(&self, state: &Self::State) -> usize {
        self.dyn_count(state)
    }
}