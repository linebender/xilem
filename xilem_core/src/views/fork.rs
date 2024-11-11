// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppendVec, ElementSplice, Mut, NoElement, View, ViewId, ViewMarker, ViewPathTracker,
    ViewSequence,
};

/// Create a view which acts as `active_view`, whilst also running `alongside_view`, without inserting it into the tree.
///
/// `alongside_view` must be a `ViewSequence` with an element type of [`NoElement`].
pub fn fork<Active, Alongside>(
    active_view: Active,
    alongside_view: Alongside,
) -> Fork<Active, Alongside> {
    Fork {
        active_view,
        alongside_view,
    }
}

/// The view for [`fork`].
#[derive(Debug)]
pub struct Fork<Active, Alongside> {
    active_view: Active,
    alongside_view: Alongside,
}

impl<Active, Alongside> ViewMarker for Fork<Active, Alongside> {}
impl<State, Action, Context, Active, Alongside, Message> View<State, Action, Context, Message>
    for Fork<Active, Alongside>
where
    Active: View<State, Action, Context, Message>,
    Alongside: ViewSequence<State, Action, Context, NoElement, Message>,
    Context: ViewPathTracker,
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

    fn rebuild(
        &self,
        prev: &Self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.active_view
                .rebuild(&prev.active_view, active_state, ctx, element);
        });
        ctx.with_id(ViewId::new(1), |ctx| {
            self.alongside_view.seq_rebuild(
                &prev.alongside_view,
                alongside_state,
                ctx,
                &mut NoElements,
            );
        });
    }

    fn teardown(
        &self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
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
/// It is technically possible for someone to create an implementation of `ViewSequence`
/// which uses a `NoElement` `ElementSplice`. But we don't think that sequence could be meaningful.
struct NoElements;

impl ElementSplice<NoElement> for NoElements {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<NoElement>) -> R) -> R {
        let mut append_vec = AppendVec::default();
        f(&mut append_vec)
    }

    fn insert(&mut self, _: NoElement) {}

    fn mutate<R>(&mut self, f: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        f(())
    }

    fn skip(&mut self, _: usize) {}

    fn delete<R>(&mut self, f: impl FnOnce(<NoElement as crate::ViewElement>::Mut<'_>) -> R) -> R {
        f(())
    }
}
