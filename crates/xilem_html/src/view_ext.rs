// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use crate::{
    class::Class, event::OptionalAction, events, view::View, Adapt, AdaptState, AdaptThunk, Event,
};

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
    ) -> events::OnClick<T, A, Self, F, OA> {
        events::on_click(self, f)
    }

    /// Add an `ondblclick` event listener.
    fn on_dblclick<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::MouseEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> events::OnDblClick<T, A, Self, F, OA> {
        events::on_dblclick(self, f)
    }

    /// Add an `oninput` event listener.
    fn on_input<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::InputEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> events::OnInput<T, A, Self, F, OA> {
        events::on_input(self, f)
    }

    /// Add an `onkeydown` event listener.
    fn on_keydown<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::KeyboardEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> events::OnKeyDown<T, A, Self, F, OA> {
        events::on_keydown(self, f)
    }

    fn on_blur<
        OA: OptionalAction<A>,
        F: Fn(&mut T, &Event<web_sys::FocusEvent, Self::Element>) -> OA,
    >(
        self,
        f: F,
    ) -> events::OnBlur<T, A, Self, F, OA> {
        events::on_blur(self, f)
    }

    fn adapt<ParentT, ParentA, F>(self, f: F) -> Adapt<ParentT, ParentA, T, A, Self, F>
    where
        F: Fn(&mut ParentT, AdaptThunk<T, A, Self>) -> xilem_core::MessageResult<ParentA>,
    {
        Adapt::new(f, self)
    }

    fn adapt_state<ParentT, F>(self, f: F) -> AdaptState<ParentT, T, Self, F>
    where
        F: Fn(&mut ParentT) -> &mut T + Send,
    {
        AdaptState::new(f, self)
    }

    /// Apply a CSS class to the child view.
    fn class(self, class: impl Into<Cow<'static, str>>) -> Class<Self> {
        crate::class::class(self, class)
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {}
