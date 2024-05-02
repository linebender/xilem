// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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
    type State = (Id, C::State);

    type Element = crate::widget::ScrollView;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (child_id, child_state, child_element)) =
            cx.with_new_id(|cx| self.child.build(cx));
        let element = crate::widget::ScrollView::new(child_element);
        (id, (child_id, child_state), element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let child_element = element.child_mut().downcast_mut().unwrap();
            self.child
                .rebuild(cx, &prev.child, &mut state.0, &mut state.1, child_element)
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match id_path {
            [child_id, rest_path @ ..] if *child_id == state.0 => {
                self.child
                    .message(rest_path, &mut state.1, message, app_state)
            }
            _ => MessageResult::Stale(message),
        }
    }
}
