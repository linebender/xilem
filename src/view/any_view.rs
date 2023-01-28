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

use crate::{event::MessageResult, id::Id, Pod, widget::{AnyWidget, ChangeFlags}};

use super::{Cx, View};

/// A trait enabling type erasure of views.
///
/// The name is slightly misleading as it's not any view, but only ones
/// whose element is AnyWidget.
///
/// Making a trait which is generic over another trait bound appears to
/// be well beyond the capability of Rust's type system. If type-erased
/// views with other bounds are needed, the best approach is probably
/// duplication of the code, probably with a macro.
pub trait AnyView<T, A = ()> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Pod);

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Pod,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<T, A, V: View<T, A> + 'static> AnyView<T, A> for V
where
    V::State: 'static,
    V::Element: AnyWidget + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Pod) {
        let (id, state, element) = self.build(cx);
        (id, Box::new(state), element)
    }

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Pod,
    ) -> ChangeFlags {
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                self.rebuild(cx, prev, id, state, element)
            } else {
                println!("downcast of state failed in dyn_rebuild");
                ChangeFlags::empty()
            }
        } else {
            let (new_id, new_state, new_element) = self.build(cx);
            *id = new_id;
            *state = Box::new(new_state);
            *element = new_element;

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
}

impl<T, A> View<T, A> for Box<dyn AnyView<T, A> + Send> {
    type State = Box<dyn Any + Send>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Pod) {
        self.deref().dyn_build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Pod,
    ) -> ChangeFlags {
        self.deref()
            .dyn_rebuild(cx, prev.deref(), id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.deref()
            .dyn_message(id_path, state.deref_mut(), message, app_state)
    }
}
