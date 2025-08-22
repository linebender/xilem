// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::cmp::PartialEq;
use std::marker::PhantomData;

use masonry::core::Property;
use xilem_core::{MessageContext, Mut};

use crate::core::{View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A view that adds a property `P` or overrides a previously defined property `P`.
pub struct Prop<P, V, State, Action> {
    pub(crate) property: P,
    pub(crate) child: V,
    pub(crate) phantom: PhantomData<fn() -> (State, Action)>,
}

impl<P, V, State, Action> ViewMarker for Prop<P, V, State, Action> {}
impl<P, Child, State, Action> View<State, Action, ViewCtx> for Prop<P, Child, State, Action>
where
    P: Property + PartialEq + Clone,
    Child: WidgetView<State, Action>,
    Child::Widget: masonry::core::HasProperty<P>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<Child::Widget>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (mut child_pod, child_state) = self.child.build(ctx, app_state);
        child_pod
            .new_widget
            .properties
            .insert(self.property.clone());
        (child_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.child.rebuild(
            &prev.child,
            view_state,
            ctx,
            element.reborrow_mut(),
            app_state,
        );
        // If a child view changed the property, we know we're out of date.
        if self.property != prev.property || element.prop_has_changed::<P>() {
            element.insert_prop(self.property.clone());
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.child.teardown(view_state, ctx, element, app_state);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        self.child.message(view_state, message, element, app_state)
    }
}
