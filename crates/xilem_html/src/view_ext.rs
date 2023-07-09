// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use crate::{class::Class, event::OptionalAction, events as e, view::View, Event};

/// A trait that makes it possible to attach event listeners and more to views
/// in the continuation style.
pub trait ViewExt<T, A>: View<T, A> + Sized {
    /// Add an `onclick` event listener.
    fn on_click<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnClick<T, A, Self, F, OA>;
    /// Add an `ondblclick` event listener.
    fn on_dblclick<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnDblClick<T, A, Self, F, OA>;
    /// Add an `oninput` event listener.
    fn on_input<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::InputEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnInput<T, A, Self, F, OA>;
    /// Add an `onkeydown` event listener.
    fn on_keydown<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::KeyboardEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnKeyDown<T, A, Self, F, OA>;
    fn on_blur<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::FocusEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnBlur<T, A, Self, F, OA>;
    /// Apply a CSS class to the child view.
    fn class(self, class: impl Into<Cow<'static, str>>) -> Class<Self> {
        crate::class::class(self, class)
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {
    fn on_click<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnClick<T, A, Self, F, OA> {
        e::on_click(self, f)
    }
    fn on_dblclick<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnDblClick<T, A, Self, F, OA> {
        e::on_dblclick(self, f)
    }
    fn on_input<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::InputEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnInput<T, A, Self, F, OA> {
        crate::events::on_input(self, f)
    }
    fn on_keydown<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::KeyboardEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnKeyDown<T, A, Self, F, OA> {
        crate::events::on_keydown(self, f)
    }
    fn on_blur<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::FocusEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> e::OnBlur<T, A, Self, F, OA> {
        e::on_blur(self, f)
    }
}
