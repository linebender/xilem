// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::{
    core::{DynMessage, View, ViewMarker},
    Affine, Pod, ViewCtx, WidgetView,
};

/// A view which transforms the widget created by child.
///
/// Each widget can only be transformed once, and using this
/// function in a nesting pattern may cause panics.
/// This is due to the efficient way that Xilem calculates field changes,
/// which is incompatible with composing non-idempotent changes to the same field.
///
/// The transform can be set using the methods on the return type.
/// Transformations apply in order.
/// That is, calling [`rotate`](Transformed::rotate) then [`translate`](Transformed::translate)
/// will move the rotated widget.
pub fn transformed<Child, State, Action>(child: Child) -> Transformed<Child, State, Action>
where
    Child: WidgetView<State, Action>,
{
    Transformed {
        child,
        transform: Affine::IDENTITY,
        phantom: PhantomData,
    }
}

/// The view for [`transformed`].
pub struct Transformed<V, State, Action> {
    child: V,
    transform: Affine,
    phantom: PhantomData<(State, Action)>,
}

impl<V, State, Action> Transformed<V, State, Action> {
    #[must_use]
    /// Rotate the widget by `radians` radians about the origin of its natural location.
    pub fn rotate(mut self, radians: f64) -> Self {
        self.transform = self.transform.then_rotate(radians);
        self
    }

    /// Scale the widget by `uniform` in each axis.
    #[must_use]
    pub fn scale(mut self, uniform: f64) -> Self {
        self.transform = self.transform.then_scale(uniform);
        self
    }

    #[must_use]
    /// Scale the widget by the given amount in each (2d) axis.
    pub fn scale_non_uniform(mut self, x: f64, y: f64) -> Self {
        self.transform = self.transform.then_scale_non_uniform(x, y);
        self
    }

    #[must_use]
    /// Displace the widget by `v` from its natural location.
    pub fn translate(mut self, v: impl Into<crate::Vec2>) -> Self {
        self.transform = self.transform.then_translate(v.into());
        self
    }

    #[must_use]
    /// Apply an arbitrary 2d transform to the widget.
    pub fn transform(mut self, v: impl Into<Affine>) -> Self {
        self.transform *= v.into();
        self
    }
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
        if child_pod.transform.is_some() {
            panic!("Tried to create a `Transformed` with an already controlled Transform");
        }
        child_pod.transform = Some(self.transform);
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
}
