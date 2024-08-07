// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_core::{MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{DomNode, DomView, DynMessage, ViewCtx};

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

/// Invokes the `callback` after the inner `element` [`DomView`] was created.
/// The callback has a reference to the raw DOM node as its only parameter.
///
/// Caution: At this point, however,
/// no properties have been applied to the node.
///
/// The use of this function should be avoided and
/// should only be utilized in exceptional cases!
pub fn after_build<E, F>(element: E, callback: F) -> AfterBuild<E, F> {
    AfterBuild { element, callback }
}

/// Invokes the `callback` after the inner `element` [`DomView<State>`] was rebuild, which usually happens after anything has changed in the `State` .
///
/// Memoization can prevent `callback` being called.
/// The callback has a reference to the raw DOM node as its only parameter.
///
/// The use of this function should be avoided and
/// should only be utilized in exceptional cases!
pub fn after_rebuild<E, F>(element: E, callback: F) -> AfterRebuild<E, F> {
    AfterRebuild { element, callback }
}

/// Invokes the `callback` before the inner `element` [`DomView`] (and its underlying DOM node) is destroyed.
/// The callback has a reference to the raw DOM node as its only parameter.
///
/// The use of this function should be avoided and
/// should only be utilized in exceptional cases!
pub fn before_teardown<E, F>(element: E, callback: F) -> BeforeTeardown<E, F> {
    BeforeTeardown { element, callback }
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
        // TODO:
        // The props should be applied before the callback is invoked.
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
        element.node.apply_props(element.props);
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
