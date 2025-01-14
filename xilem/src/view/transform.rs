// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::Affine;
use xilem_core::{DynMessage, View, ViewMarker};

use crate::{Pod, ViewCtx, WidgetView};

/// A view which transforms `child`.
///
/// The transform can be set using the methods on `Transformable`.
pub fn transforming<Child, State, Action>(child: Child) -> Transformed<Child, State, Action>
where
    Child: WidgetView<State, Action>,
{
    Transformed {
        child,
        transform: Affine::IDENTITY,
        phantom: PhantomData,
    }
}

pub struct Transformed<V, State, Action> {
    child: V,
    transform: Affine,
    phantom: PhantomData<(State, Action)>,
}

impl<V, State, Action> ViewMarker for Transformed<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for Transformed<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<Child::Widget>;
    type ViewState = Child::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut child_pod, child_state) = self.child.build(ctx);
        // TODO: Use a marker identity value to detect this more properly
        if child_pod.transform != Affine::IDENTITY {
            panic!("Tried to create a `Transformed` with an already controlled Transform");
        }
        child_pod.transform = self.transform;
        (child_pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: xilem_core::Mut<'_, Self::Element>,
    ) {
        if self.transform != prev.transform {
            element.ctx.set_transform(self.transform);
        }
        self.child.rebuild(&prev.child, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem_core::Mut<'_, Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action, DynMessage> {
        self.child.message(view_state, id_path, message, app_state)
    }
}

/// An extension trait, to allow common transformations of the views transform.
pub trait Transformable: Sized {
    fn transform_mut(&mut self) -> &mut Affine;

    #[must_use]
    fn rotate(mut self, radians: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_rotate(radians);
        self
    }

    #[must_use]
    fn scale(mut self, uniform: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_scale(uniform);
        self
    }

    #[must_use]
    fn scale_non_uniform(mut self, x: f64, y: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_scale_non_uniform(x, y);
        self
    }

    #[must_use]
    fn translate(mut self, v: impl Into<crate::Vec2>) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_translate(v.into());
        self
    }

    #[must_use]
    fn transform(mut self, v: impl Into<Affine>) -> Self {
        *self.transform_mut() *= v.into();
        self
    }
}
