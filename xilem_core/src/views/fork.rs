// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{
    AppendVec, ElementSplice, Mut, NoElement, View, ViewId, ViewPathTracker, ViewSequence,
};

/// Create a view which acts as `active_view`, whilst also running `alongside_view`, without inserting it into the tree.
///
/// `alongside_view` must be a `ViewSequence` with an element type of [`NoElement`].
pub fn fork<Active, Alongside, Marker>(
    active_view: Active,
    alongside_view: Alongside,
) -> Fork<Active, Alongside, Marker> {
    Fork {
        active_view,
        alongside_view,
        marker: PhantomData,
    }
}

/// The view for [`fork`].
pub struct Fork<Active, Alongside, Marker> {
    active_view: Active,
    alongside_view: Alongside,
    marker: PhantomData<Marker>,
}

impl<State, Action, Context, Active, Alongside, Marker, Message>
    View<State, Action, Context, Message> for Fork<Active, Alongside, Marker>
where
    Active: View<State, Action, Context, Message>,
    Alongside: ViewSequence<State, Action, Context, NoElement, Marker, Message>,
    Context: ViewPathTracker,
    Marker: 'static,
{
    type Element = Active::Element;

    type ViewState = (Active::ViewState, Alongside::SeqState);

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let (element, active_state) =
            ctx.with_id(ViewId::new(0), |ctx| self.active_view.build(ctx));
        let alongside_state = ctx.with_id(ViewId::new(1), |ctx| {
            self.alongside_view
                .seq_build(ctx, &mut AppendVec::default())
        });
        (element, (active_state, alongside_state))
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let element = ctx.with_id(ViewId::new(0), |ctx| {
            self.active_view
                .rebuild(&prev.active_view, active_state, ctx, element)
        });
        ctx.with_id(ViewId::new(1), |ctx| {
            self.alongside_view.seq_rebuild(
                &prev.alongside_view,
                alongside_state,
                ctx,
                &mut NoElements,
            );
        });
        element
    }

    fn teardown(
        &self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.alongside_view
                .seq_teardown(alongside_state, ctx, &mut NoElements);
        });
        ctx.with_id(ViewId::new(1), |ctx| {
            self.active_view.teardown(active_state, ctx, element);
        });
    }

    fn message(
        &self,
        (active_state, alongside_state): &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Message,
        app_state: &mut State,
    ) -> crate::MessageResult<Action, Message> {
        let (first, id_path) = id_path
            .split_first()
            .expect("Id path has elements for Fork");
        match first.routing_id() {
            0 => self
                .active_view
                .message(active_state, id_path, message, app_state),
            1 => self
                .alongside_view
                .seq_message(alongside_state, id_path, message, app_state),
            _ => unreachable!(),
        }
    }
}

/// A stub `ElementSplice` implementation for `NoElement`.
///
/// We know that none of the methods will be called, because the `ViewSequence`
/// implementation for `NoElement` views does not use the provided `elements`.
///
/// It is technically possible for someone to create an implementation of `ViewSequence`
/// which uses a `NoElement` `ElementSplice`. But we don't think that sequence could be meaningful,
/// so we still panic in that case.
struct NoElements;

impl ElementSplice<NoElement> for NoElements {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<NoElement>) -> R) -> R {
        let mut append_vec = AppendVec::default();
        let ret = f(&mut append_vec);
        debug_assert!(append_vec.into_inner().is_empty());
        ret
    }

    fn insert(&mut self, _: NoElement) {
        unreachable!()
    }

    fn mutate<R>(&mut self, _: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        unreachable!()
    }

    fn skip(&mut self, n: usize) {
        if n > 0 {
            unreachable!()
        }
    }

    fn delete<R>(&mut self, _: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        unreachable!()
    }
}
