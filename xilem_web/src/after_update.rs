// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_core::{MessageResult, Mut, View, ViewId, ViewPathTracker};

use crate::{interfaces::Element, DynMessage, ViewCtx};

pub struct AfterUpdate<E, F> {
    pub(crate) element: E,
    pub(crate) callback: F,
}

impl<E, F> AfterUpdate<E, F> {
    pub fn new(element: E, callback: F) -> AfterUpdate<E, F> {
        AfterUpdate { element, callback }
    }
}

pub struct AfterUpdateState<E, S> {
    element: E,
    child_state: S,
    child_id: ViewId,
}

impl<State, Action, E, F> View<State, Action, ViewCtx, DynMessage> for AfterUpdate<E, F>
where
    State: 'static,
    Action: 'static,
    E: Element<State, Action>,
    E::Element: Clone + PartialEq,
    F: Fn(&mut State, &E::Element) + 'static,
{
    type Element = E::Element;

    type ViewState = AfterUpdateState<Self::Element, E::ViewState>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let id = ViewId::new(0); // FIXME: what's the right number here?
        ctx.with_id(id, |ctx| {
            let (el, child_state) = self.element.build(ctx);
            let child_id = ViewId::new(1); // FIXME: where to get the child ID from?
            let element = el.clone();
            let state = AfterUpdateState {
                child_state,
                child_id,
                element,
            };
            let id_path = ctx.view_path().to_vec();
            ctx.after_update
                .insert(*id_path.last().unwrap(), (true, id_path));
            (el, state)
        })
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let id = ViewId::new(0); // FIXME: what's the right number here?
        ctx.with_id(id, |ctx| {
            let rebuild_outcome =
                self.element
                    .rebuild(&prev.element, &mut view_state.child_state, ctx, element);

            // FIXME:
            // if *element != view_state.element {
            //     view_state.element = element.clone();
            // }

            let view_path = ctx.view_path().to_vec();
            ctx.after_update
                .entry(id)
                .and_modify(|e| e.0 = true)
                .or_insert((true, view_path));

            rebuild_outcome
        })
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {
        // FIXME
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        match id_path {
            [] => {
                (self.callback)(app_state, &view_state.element);
                MessageResult::Nop
            }
            [element_id, rest_path @ ..] if *element_id == view_state.child_id => self
                .element
                .message(&mut view_state.child_state, rest_path, message, app_state),
            _ => MessageResult::Stale(message),
        }
    }
}
