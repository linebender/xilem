// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_core::{MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{DomView, DynMessage, ViewCtx};

pub struct AfterBuild<E, F> {
    element: E,
    callback: F,
}

pub struct AfterRebuild<E, F> {
    element: E,
    callback: F,
}

pub struct BeforeTeardown<E, F> {
    element: E,
    callback: F,
}

impl<E, F> AfterBuild<E, F> {
    pub fn new(element: E, callback: F) -> AfterBuild<E, F> {
        Self { element, callback }
    }
}

impl<E, F> AfterRebuild<E, F> {
    pub fn new(element: E, callback: F) -> AfterRebuild<E, F> {
        Self { element, callback }
    }
}

impl<E, F> BeforeTeardown<E, F> {
    pub fn new(element: E, callback: F) -> BeforeTeardown<E, F> {
        Self { element, callback }
    }
}

impl<E, F> ViewMarker for AfterBuild<E, F> {}
impl<E, F> ViewMarker for AfterRebuild<E, F> {}
impl<E, F> ViewMarker for BeforeTeardown<E, F> {}

impl<State, V, F> View<State, (), ViewCtx, DynMessage> for AfterBuild<V, F>
where
    State: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (el, view_state) = self.element.build(ctx);
        (self.callback)(&el.node);
        (el, view_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        self.element
            .rebuild(&prev.element, view_state, ctx, element)
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<'_, Self::Element>,
    ) {
        self.element.teardown(view_state, ctx, el);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<(), DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}

impl<State, V, F> View<State, (), ViewCtx, DynMessage> for AfterRebuild<V, F>
where
    State: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        self.element.build(ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let element = self
            .element
            .rebuild(&prev.element, view_state, ctx, element);
        (self.callback)(element.node);
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<'_, Self::Element>,
    ) {
        self.element.teardown(view_state, ctx, el);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<(), DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}

impl<State, V, F> View<State, (), ViewCtx, DynMessage> for BeforeTeardown<V, F>
where
    State: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        self.element.build(ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        self.element
            .rebuild(&prev.element, view_state, ctx, element)
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<'_, Self::Element>,
    ) {
        (self.callback)(el.node);
        self.element.teardown(view_state, ctx, el);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<(), DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}
