// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use xilem_core::MessageResult;

use crate::{class::Class, events as e, view::View, Event};

pub trait ViewExt<T, A>: View<T, A> + Sized {
    fn on_click<F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> MessageResult<A>>(
        self,
        f: F,
    ) -> e::OnClick<Self, F>;
    fn on_dblclick<F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> MessageResult<A>>(
        self,
        f: F,
    ) -> e::OnDblClick<Self, F>;
    fn on_input<F: Fn(&mut T, &Event<web_sys::InputEvent, Self::Element>) -> MessageResult<A>>(
        self,
        f: F,
    ) -> e::OnInput<Self, F>;
    fn on_keydown<
        F: Fn(&mut T, &Event<web_sys::KeyboardEvent, Self::Element>) -> MessageResult<A>,
    >(
        self,
        f: F,
    ) -> e::OnKeyDown<Self, F>;
    fn class(self, class: impl Into<Cow<'static, str>>) -> Class<Self> {
        crate::class::class(self, class)
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {
    fn on_click<F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> MessageResult<A>>(
        self,
        f: F,
    ) -> e::OnClick<Self, F> {
        e::on_click(self, f)
    }
    fn on_dblclick<
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> MessageResult<A>,
    >(
        self,
        f: F,
    ) -> e::OnDblClick<Self, F> {
        e::on_dblclick(self, f)
    }
    fn on_input<F: Fn(&mut T, &Event<web_sys::InputEvent, Self::Element>) -> MessageResult<A>>(
        self,
        f: F,
    ) -> e::OnInput<Self, F> {
        crate::events::on_input(self, f)
    }
    fn on_keydown<
        F: Fn(&mut T, &Event<web_sys::KeyboardEvent, Self::Element>) -> MessageResult<A>,
    >(
        self,
        f: F,
    ) -> e::OnKeyDown<Self, F> {
        crate::events::on_keydown(self, f)
    }
}
