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

use crate::{view::ViewMarker, widget::AsAny};
use crate::{
    event::MessageResult,
    id::Id,
    widget::ChangeFlags,
};

use super::{
    view::GenericView,
    Cx, TraitBound,
};

/// A trait enabling type erasure of views.
///
/// The name is slightly misleading as it's not any view, but only ones
/// whose element is AnyWidget.
///
/// Making a trait which is generic over another trait bound appears to
/// be well beyond the capability of Rust's type system. If type-erased
/// views with other bounds are needed, the best approach is probably
/// duplication of the code, probably with a macro.
pub trait AnyView<T, W: ?Sized, A = ()> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Box<W>);

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, W, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Box<W>,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<T, W: ?Sized, A, V: GenericView<T, W, A> + 'static> AnyView<T, W, A> for V
where
    V::State: 'static,
    V::Element: TraitBound<W> + 'static,
    Box<W>: AsAny,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Box<W>) {
        let (id, state, element) = self.build(cx);
        (id, Box::new(state), element.boxed())
    }

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, W, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Box<W>,
    ) -> ChangeFlags {
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                if let Some(element) = element.as_any_mut().downcast_mut() {
                    self.rebuild(cx, prev, id, state, element)
                } else {
                    println!("downcast of element failed in dyn_rebuild");
                    ChangeFlags::default()
                }
            } else {
                println!("downcast of state failed in dyn_rebuild");
                ChangeFlags::default()
            }
        } else {
            let (new_id, new_state, new_element) = self.build(cx);
            *id = new_id;
            *state = Box::new(new_state);
            *element = new_element.boxed();

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

impl<T, W: ?Sized, A> ViewMarker for Box<dyn AnyView<T, W, A> + Send> {}

impl<T, W: ?Sized, A> GenericView<T, W, A> for Box<dyn AnyView<T, W, A> + Send>
    where Box<W>: TraitBound<W>
{
    type State = Box<dyn Any + Send>;

    type Element = Box<W>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        self.deref().dyn_build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
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
