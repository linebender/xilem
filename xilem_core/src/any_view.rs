// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for a type erased [`View`].

use core::any::Any;

use alloc::boxed::Box;

use crate::{AnyElement, DynMessage, MessageResult, View, ViewId, ViewPathTracker};

/// A view which can have any view type where the [`View::Element`] is compatible with
/// `Element`.
///
/// This is primarily used for type erasure of views.
///
/// This is useful for a view which can be any of several view types, by using
/// `Box<dyn AnyView<...>>`, which implements [`View`].
// TODO: Mention `Either` when we have implemented that?
///
/// This is also useful for memoization, by storing an `Option<Arc<dyn AnyView<...>>>`,
/// then [inserting](Option::get_or_insert_with) into that option at view tree construction time.
///
/// Libraries using `xilem_core` are expected to have a type alias for their own `AnyView`, which specifies
/// the `Context` and `Element` types.
pub trait AnyView<State, Action, Context, Element: crate::ViewElement> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, ctx: &mut Context) -> (Element, AnyViewState);

    fn dyn_rebuild(
        &self,
        dyn_state: &mut AnyViewState,
        ctx: &mut Context,
        prev: &dyn AnyView<State, Action, Context, Element>,
        element: Element::Mut<'_>,
    );

    /// Returns `Element::Mut<'el>` so that the element can be
    /// returned and replaced in `dyn_rebuild`, if needed
    fn dyn_teardown<'el>(
        &self,
        dyn_state: &mut AnyViewState,
        ctx: &mut Context,
        element: Element::Mut<'el>,
    ) -> Element::Mut<'el>;

    fn dyn_message(
        &self,
        dyn_state: &mut AnyViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

impl<State, Action, Context, DynamicElement, V> AnyView<State, Action, Context, DynamicElement>
    for V
where
    DynamicElement: AnyElement<V::Element>,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + 'static,
    V::ViewState: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_build(&self, ctx: &mut Context) -> (DynamicElement, AnyViewState) {
        let generation = 0;
        let (element, view_state) = ctx.with_id(ViewId::new(generation), |ctx| self.build(ctx));
        (
            DynamicElement::upcast(element),
            AnyViewState {
                inner_state: Box::new(view_state),
                generation,
            },
        )
    }

    fn dyn_rebuild(
        &self,
        dyn_state: &mut AnyViewState,
        ctx: &mut Context,
        prev: &dyn AnyView<State, Action, Context, DynamicElement>,
        mut element: <DynamicElement as crate::ViewElement>::Mut<'_>,
    ) {
        if let Some(prev) = prev.as_any().downcast_ref() {
            // If we were previously of this type, then do a normal rebuild
            DynamicElement::with_downcast(element, |element| {
                let state = dyn_state
                    .inner_state
                    .downcast_mut()
                    .expect("build or rebuild always set the correct corresponding state type");

                ctx.with_id(ViewId::new(dyn_state.generation), move |ctx| {
                    self.rebuild(prev, state, ctx, element);
                });
            });
        } else {
            // Otherwise, teardown the old element, then replace the value
            element = prev.dyn_teardown(dyn_state, ctx, element);

            // Increase the generation, because the underlying widget has been swapped out.
            // Overflow condition: Impossible to overflow, as u64 only ever incremented by 1
            // and starting at 0.
            dyn_state.generation = dyn_state.generation.wrapping_add(1);
            let (new_element, view_state) =
                ctx.with_id(ViewId::new(dyn_state.generation), |ctx| self.build(ctx));
            dyn_state.inner_state = Box::new(view_state);
            DynamicElement::replace_inner(element, new_element);
        }
    }
    fn dyn_teardown<'el>(
        &self,
        dyn_state: &mut AnyViewState,
        ctx: &mut Context,
        element: <DynamicElement as crate::ViewElement>::Mut<'el>,
    ) -> <DynamicElement as crate::ViewElement>::Mut<'el> {
        let state = dyn_state
            .inner_state
            .downcast_mut()
            .expect("build or rebuild always set the correct corresponding state type");

        // We only need to teardown the inner value - there's no other state to cleanup in this widget
        DynamicElement::with_downcast(element, |element| {
            ctx.with_id(ViewId::new(dyn_state.generation), |ctx| {
                self.teardown(state, ctx, element);
            });
        })
    }

    fn dyn_message(
        &self,
        dyn_state: &mut AnyViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let state = dyn_state
            .inner_state
            .downcast_mut()
            .expect("build or rebuild always set the correct corresponding state type");
        let Some((first, remainder)) = id_path.split_first() else {
            unreachable!("Parent view of `AnyView` sent outdated and/or incorrect empty view path");
        };
        if first.routing_id() != dyn_state.generation {
            // Do we want to log something here?
            return MessageResult::Stale(message);
        }
        self.message(state, remainder, message, app_state)
    }
}

/// The state used by [`AnyView`].
#[doc(hidden)]
pub struct AnyViewState {
    inner_state: Box<dyn Any>,
    /// The generation is the value which is shown
    generation: u64,
}

impl<State: 'static, Action: 'static, Context, Element> View<State, Action, Context>
    for dyn AnyView<State, Action, Context, Element>
where
    // Element must be `static` so it can be downcasted
    Element: crate::ViewElement + 'static,
    Context: crate::ViewPathTracker + 'static,
{
    type Element = Element;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.dyn_build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_rebuild(view_state, ctx, prev, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.dyn_message(view_state, id_path, message, app_state)
    }
}

// TODO: IWBN if we could avoid this
impl<State: 'static, Action: 'static, Context, Element> View<State, Action, Context>
    for dyn AnyView<State, Action, Context, Element> + Send
where
    // Element must be `static` so it can be downcasted
    Element: crate::ViewElement + 'static,
    Context: crate::ViewPathTracker + 'static,
{
    type Element = Element;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.dyn_build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_rebuild(view_state, ctx, prev, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.dyn_message(view_state, id_path, message, app_state)
    }
}

impl<State: 'static, Action: 'static, Context, Element> View<State, Action, Context>
    for dyn AnyView<State, Action, Context, Element> + Send + Sync
where
    // Element must be `static` so it can be downcasted
    Element: crate::ViewElement + 'static,
    Context: crate::ViewPathTracker + 'static,
{
    type Element = Element;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.dyn_build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_rebuild(view_state, ctx, prev, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.dyn_message(view_state, id_path, message, app_state)
    }
}

impl<State: 'static, Action: 'static, Context, Element> View<State, Action, Context>
    for dyn AnyView<State, Action, Context, Element> + Sync
where
    // Element must be `static` so it can be downcasted
    Element: crate::ViewElement + 'static,
    Context: crate::ViewPathTracker + 'static,
{
    type Element = Element;

    type ViewState = AnyViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.dyn_build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_rebuild(view_state, ctx, prev, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::ViewElement>::Mut<'_>,
    ) {
        self.dyn_teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.dyn_message(view_state, id_path, message, app_state)
    }
}
