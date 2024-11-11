// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewId, ViewMarker},
    DomView, DynMessage, PodMut, ViewCtx,
};
use std::{any::TypeId, rc::Rc};
use wasm_bindgen::UnwrapThrowExt;

/// This view creates an internally cached deep-clone of the underlying DOM node. When the inner view is created again, this will be done more efficiently.
pub struct Templated<V>(Rc<V>);

impl<V> ViewMarker for Templated<V> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Templated<V>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action>,
{
    type Element = V::Element;

    type ViewState = <Rc<V> as View<State, Action, ViewCtx, DynMessage>>::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let type_id = TypeId::of::<Self>();
        let (element, view_state) = if let Some((template_node, view)) = ctx.templates.get(&type_id)
        {
            let prev = view.clone();
            let prev = prev.downcast_ref::<Rc<V>>().unwrap_throw();
            let node = template_node.clone_node_with_deep(true).unwrap_throw();
            let (mut el, mut state) = ctx.with_hydration_node(node, |ctx| prev.build(ctx));
            el.apply_changes();
            let pod_mut = PodMut::new(&mut el.node, &mut el.props, &mut el.flags, None, false);
            self.0.rebuild(prev, &mut state, ctx, pod_mut);

            (el, state)
        } else {
            let (element, state) = self.0.build(ctx);

            let template: web_sys::Node = element
                .node
                .as_ref()
                .clone_node_with_deep(true)
                .unwrap_throw();

            ctx.templates
                .insert(type_id, (template, Rc::new(self.0.clone())));
            (element, state)
        };
        (element, view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.0.rebuild(&prev.0, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.0.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.0.message(view_state, id_path, message, app_state)
    }
}

/// This view creates an internally cached deep-clone of the underlying DOM node.
///
/// When the inner view is created again, this will be done more efficiently.
/// It's recommended to use this as wrapper, when it's expected that the inner `view` is a little bigger and will be created a lot, for example in a long list
/// It's *not* recommended to use this, when the inner `view` is rather small (as in the example),
/// as it in that case introduces a little bit of overhead (memory and perf)
///
/// Additionally it supports memoization when the given `view` is an [`Rc<impl DomView>`].
///
/// # Examples
///
/// ```
/// use xilem_web::{templated, elements::html, DomFragment};
///
/// fn long_list_fragment() -> impl DomFragment<()> {
///     (0..1000)
///         // Performance increase will be larger with a deeper child views
///         .map(|num| templated(html::li(format!("number: {num}"))))
///         .collect::<Vec<_>>()
/// }
/// ```
pub fn templated<State, Action, E>(view: impl Into<Rc<E>>) -> Templated<E>
where
    E: DomView<State, Action>,
{
    Templated(view.into())
}
