// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::TypeId, ops::Deref as _, rc::Rc};

use wasm_bindgen::UnwrapThrowExt;
use xilem_core::{MessageResult, View, ViewMarker};

use crate::{DomNode, DomView, DynMessage, PodMut, ViewCtx};

/// This view creates an internally cached deep-clone of the underlying DOM node. When the inner view is created again, this will be done more efficiently.
pub struct Templated<E>(Rc<E>);

pub struct TemplatedState<ViewState> {
    view_state: ViewState,
    dirty: bool,
}

impl<E> ViewMarker for Templated<E> {}
impl<State, Action, E> View<State, Action, ViewCtx, DynMessage> for Templated<E>
where
    State: 'static,
    Action: 'static,
    E: DomView<State, Action>,
{
    type Element = E::Element;

    type ViewState = TemplatedState<E::ViewState>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let type_id = TypeId::of::<Self>();
        let (element, view_state) = if let Some((template_node, view)) = ctx.templates.get(&type_id)
        {
            let prev = view.clone();
            let prev = prev.downcast_ref::<E>().unwrap_throw();
            let node = template_node.clone_node_with_deep(true).unwrap_throw();
            let is_already_hydrating = ctx.is_hydrating();
            ctx.enable_hydration();
            ctx.push_hydration_node(node);
            let (mut el, mut state) = prev.build(ctx);
            el.node.apply_props(&mut el.props);
            if !is_already_hydrating {
                ctx.disable_hydration();
            }
            let pod_mut = PodMut::new(&mut el.node, &mut el.props, None, false);
            self.0.rebuild(prev, &mut state, ctx, pod_mut);

            (el, state)
        } else {
            let (element, state) = self.0.build(ctx);

            let template: web_sys::Node = element
                .node
                .as_ref()
                .clone_node_with_deep(true)
                .unwrap_throw();

            ctx.templates.insert(type_id, (template, self.0.clone()));
            (element, state)
        };
        let state = TemplatedState {
            view_state,
            dirty: false,
        };
        (element, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem_core::Mut<'el, Self::Element>,
    ) -> xilem_core::Mut<'el, Self::Element> {
        if core::mem::take(&mut view_state.dirty) || !Rc::ptr_eq(&self.0, &prev.0) {
            self.0
                .deref()
                .rebuild(&prev.0, &mut view_state.view_state, ctx, element)
        } else {
            // If this is the same value, or no rebuild was forced, there's no need to rebuild
            element
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem_core::Mut<'_, Self::Element>,
    ) {
        self.0.teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action, DynMessage> {
        let message_result =
            self.0
                .deref()
                .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
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
