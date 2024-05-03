// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use crate::view::{Id, ViewMarker};
use crate::widget::ChangeFlags;

use super::{Cx, View};

impl ViewMarker for String {}

impl<T, A> View<T, A> for String {
    type State = ();

    type Element = crate::widget::TextWidget;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) =
            cx.with_new_id(|_| crate::widget::TextWidget::new(Cow::from(self.clone())));
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
            element.set_text(Cow::from(self.clone()))
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

impl ViewMarker for &'static str {}

impl<T, A> View<T, A> for &'static str {
    type State = ();

    type Element = crate::widget::TextWidget;

    fn build(&self, cx: &mut Cx) -> (crate::view::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| crate::widget::TextWidget::new(Cow::from(*self)));
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
            element.set_text(Cow::from(*self))
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

impl ViewMarker for Cow<'static, str> {}

impl<T, A> View<T, A> for Cow<'static, str> {
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
