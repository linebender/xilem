// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::{
    core::{MessageResult, Mut, View, ViewId, ViewMarker},
    DomNode, DomView, DynMessage, ViewCtx,
};

/// Invokes the `callback` after the inner `element` [`DomView`] was created.
/// See [`after_build`] for more details.
pub struct AfterBuild<State, Action, E, F> {
    element: E,
    callback: F,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Invokes the `callback` after the inner `element` [`DomView<State>`]
/// See [`after_rebuild`] for more details.
pub struct AfterRebuild<State, Action, E, F> {
    element: E,
    callback: F,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Invokes the `callback` before the inner `element` [`DomView`] (and its underlying DOM node) is destroyed.
/// See [`before_teardown`] for more details.
pub struct BeforeTeardown<State, Action, E, F> {
    element: E,
    callback: F,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Invokes the `callback` after the inner `element` [`DomView`] was created.
/// The callback has a reference to the raw DOM node as its only parameter.
///
/// Caution: At this point, however,
/// no properties have been applied to the node.
///
/// As accessing the underlying raw DOM node can mess with the inner logic of `xilem_web`,
/// this should only be used as an escape-hatch for properties not supported by `xilem_web`.
/// E.g. to be interoperable with external javascript libraries.
pub fn after_build<State, Action, E, F>(element: E, callback: F) -> AfterBuild<State, Action, E, F>
where
    State: 'static,
    Action: 'static,
    E: DomView<State, Action> + 'static,
    F: Fn(&E::DomNode) + 'static,
{
    AfterBuild {
        element,
        callback,
        phantom: PhantomData,
    }
}

/// Invokes the `callback` after the inner `element` [`DomView<State>`]
/// was rebuild, which usually happens after anything has changed in the `State` .
///
/// Memoization can prevent `callback` being called.
/// The callback has a reference to the raw DOM node as its only parameter.
///
/// As accessing the underlying raw DOM node can mess with the inner logic of `xilem_web`,
/// this should only be used as an escape-hatch for properties not supported by `xilem_web`.
/// E.g. to be interoperable with external javascript libraries.
pub fn after_rebuild<State, Action, E, F>(
    element: E,
    callback: F,
) -> AfterRebuild<State, Action, E, F>
where
    State: 'static,
    Action: 'static,
    E: DomView<State, Action> + 'static,
    F: Fn(&E::DomNode) + 'static,
{
    AfterRebuild {
        element,
        callback,
        phantom: PhantomData,
    }
}

/// Invokes the `callback` before the inner `element` [`DomView`] (and its underlying DOM node) is destroyed.
///
/// As accessing the underlying raw DOM node can mess with the inner logic of `xilem_web`,
/// this should only be used as an escape-hatch for properties not supported by `xilem_web`.
/// E.g. to be interoperable with external javascript libraries.
pub fn before_teardown<State, Action, E, F>(
    element: E,
    callback: F,
) -> BeforeTeardown<State, Action, E, F>
where
    State: 'static,
    Action: 'static,
    E: DomView<State, Action> + 'static,
    F: Fn(&E::DomNode) + 'static,
{
    BeforeTeardown {
        element,
        callback,
        phantom: PhantomData,
    }
}

impl<State, Action, E, F> ViewMarker for AfterBuild<State, Action, E, F> {}
impl<State, Action, E, F> ViewMarker for AfterRebuild<State, Action, E, F> {}
impl<State, Action, E, F> ViewMarker for BeforeTeardown<State, Action, E, F> {}

impl<State, Action, V, F> View<State, Action, ViewCtx, DynMessage>
    for AfterBuild<State, Action, V, F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State, Action> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut el, view_state) = self.element.build(ctx);
        el.node.apply_props(&mut el.props);
        (self.callback)(&el.node);
        (el, view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.element
            .rebuild(&prev.element, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<Self::Element>,
    ) {
        self.element.teardown(view_state, ctx, el);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}

impl<State, Action, V, F> View<State, Action, ViewCtx, DynMessage>
    for AfterRebuild<State, Action, V, F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State, Action> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        self.element.build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        self.element
            .rebuild(&prev.element, view_state, ctx, element.reborrow());
        element.node.apply_props(element.props);
        (self.callback)(element.node);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<Self::Element>,
    ) {
        self.element.teardown(view_state, ctx, el);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}

impl<State, Action, V, F> View<State, Action, ViewCtx, DynMessage>
    for BeforeTeardown<State, Action, V, F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&V::DomNode) + 'static,
    V: DomView<State, Action> + 'static,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        self.element.build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.element
            .rebuild(&prev.element, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        el: Mut<Self::Element>,
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
    ) -> MessageResult<Action, DynMessage> {
        self.element
            .message(view_state, id_path, message, app_state)
    }
}
