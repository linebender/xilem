// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::kurbo::{Affine, Vec2};

use crate::core::{Arg, MessageCtx, Mut, View, ViewArgument, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

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
    State: ViewArgument,
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
    pub fn translate(mut self, v: impl Into<Vec2>) -> Self {
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

mod private {
    use masonry::kurbo::Affine;

    /// The View state for the [Transformed](super::Transformed)
    #[expect(
        unnameable_types,
        reason = "This type has no public API, and is only public due to trait visibility rules"
    )]
    pub struct TransformedState<ChildState> {
        pub(super) child: ChildState,
        pub(super) previous_transform: Affine,
    }
}

impl<V, State, Action> ViewMarker for Transformed<V, State, Action> {}
impl<Child, State, Action> View<State, Action, ViewCtx> for Transformed<Child, State, Action>
where
    Child: WidgetView<State, Action>,
    State: ViewArgument,
    Action: 'static,
{
    type Element = Pod<Child::Widget>;
    type ViewState = private::TransformedState<Child::ViewState>;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (mut child_pod, child_state) = self.child.build(ctx, app_state);
        let state = private::TransformedState {
            child: child_state,
            previous_transform: child_pod.new_widget.options.transform,
        };
        child_pod.new_widget.options.transform = self.transform * state.previous_transform;
        (child_pod, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        self.child.rebuild(
            &prev.child,
            &mut view_state.child,
            ctx,
            element.reborrow_mut(),
            app_state,
        );
        let transform_changed = element.ctx.transform_has_changed();
        // If the child view changed the transform, we know we're out of date.
        if transform_changed {
            // If it has changed the transform, then we know that it will only be due to effects
            // "below us" (that is, it will have restarted from scratch).
            // We update our stored understanding of the transforms below us
            view_state.previous_transform = element.ctx.transform();
            // This is a convention used to communicate with ourselves, which could
            // break down if any other view handles transforms differently.
            // However, we document against this in `Pod::transform`.
        }
        if self.transform != prev.transform || transform_changed {
            element
                .ctx
                .set_transform(self.transform * view_state.previous_transform);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(&mut view_state.child, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> xilem_core::MessageResult<Action> {
        self.child
            .message(&mut view_state.child, message, element, app_state)
    }
}
