// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::any::Any;

use alloc::boxed::Box;

use crate::{MessageResult, SuperElement, View, ViewPathTracker};

pub struct AnyViewState {
    inner_state: Box<dyn Any>,
    generation: u64,
}

/// A view which can have any view type where the [`View::Element`] is compatible with
/// `Element`.
///
/// This is primarily used for type erasure of views.
///
/// This is useful for a view which can be either of two view types, or
// TODO: Mention `Either` when we have implemented that?
pub trait AnyView<State, Action, Context, Element> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, cx: &mut Context) -> (Element, AnyViewState);

    fn dyn_rebuild(
        &self,
        dyn_state: &mut AnyViewState,
        cx: &mut Context,
        prev: &dyn AnyMasonryView<T, A>,
        element: WidgetMut<DynWidget>,
    );

    fn dyn_message(
        &self,
        dyn_state: &mut AnyViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<State, Action, Context, DynamicElement, V> AnyView<State, Action, Context, DynamicElement>
    for V
where
    DynamicElement: SuperElement<V::Element>,
    Context: ViewPathTracker,
    V: View<State, Action, Context>,
{
}

impl<State, Action, Context, Element> View<State, Action, Context>
    for dyn AnyView<State, Action, Context, Element>
where
    Element: crate::Element,
    Context: crate::ViewPathTracker,
{
    type Element = Element;

    type ViewState = Box<dyn Any>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        todo!()
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::Element>::Mut<'_>,
    ) {
        todo!()
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as crate::Element>::Mut<'_>,
    ) {
        todo!()
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        todo!()
    }
}
