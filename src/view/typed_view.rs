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
use crate::{Element, Id, MessageResult, VecSplice};
use crate::view::{Cx, View, ViewSequence};
use crate::widget::ChangeFlags;

/// A typed view object representing a node in the UI.
///
/// This trait is the main trait to implement Views. For composing them take a look at [`View`].
/// It is also possible to implement View and ViewSequence directly but doing this involves more
/// boilerplate code and should only be done when you need to change your element type at runtime
/// like `AnyView`.
///
/// [``]: crate::view::View
pub trait TypedView<E: Element, T, A = ()>: Send + ViewMarker where E: From<Self::Element> {
    /// Associated state for the view.
    type State: Send;

    /// The associated widget for the view.
    type Element;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

pub trait ViewMarker {}

impl<E: Element, T, A, V: TypedView<E, T, A> + ViewMarker> View<E, T, A> for V
where
    E: From<<V as TypedView<E, T, A>>::Element>,
    <V as TypedView<E, T, A>>::Element: 'static,
{}

// ViewMarker is already a dependency of View but Rusts orphan rules dont work if we remove it here.
impl<E: Element, T, A, V: TypedView<E, T, A> + ViewMarker> ViewSequence<E, T, A> for V
    where
        E: From<<V as TypedView<E, T, A>>::Element>,
        <V as TypedView<E, T, A>>::Element: 'static,
{
    type State = (<V as TypedView<E, T, A>>::State, Id);

    fn build(&self, cx: &mut Cx, elements: &mut Vec<E>) -> Self::State {
        let (id, state, element) = <V as TypedView<E, T, A>>::build(self, cx);
        elements.push(E::from(element));
        (state, id)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut VecSplice<E>,
    ) -> ChangeFlags {
        let el = element.mutate();
        let downcast = el.downcast_mut().unwrap();
        let flags =
            <V as TypedView<E, T, A>>::rebuild(self, cx, prev, &mut state.1, &mut state.0, downcast);

        el.mark(flags)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == &state.1 {
                return <V as TypedView<E, T, A>>::message(
                    self,
                    rest_path,
                    &mut state.0,
                    message,
                    app_state,
                );
            }
        }
        MessageResult::Stale(message)
    }

    fn count(&self, _state: &Self::State) -> usize {
        1
    }
}