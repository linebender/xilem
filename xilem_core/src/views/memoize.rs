// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, Mut, View, ViewId, ViewPathTracker};

/// A view which supports Memoization.
///
/// The story of Memoization in Xilem is still being worked out,
/// so the details of this view might change.
pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}

pub struct MemoizeState<State, Action, Context, V>
where
    Context: ViewPathTracker,
    V: View<State, Action, Context>,
{
    view: V,
    view_state: V::ViewState,
    dirty: bool,
}

impl<D, V, F> Memoize<D, F>
where
    F: Fn(&D) -> V,
{
    const ASSERT_CONTEXTLESS_FN: () = {
        assert!(
            core::mem::size_of::<F>() == 0,
            "
It's not possible to use function pointers or captured context in closures,
as this potentially messes up the logic of memoize or produces unwanted effects.

For example a different kind of view could be instantiated with a different callback, while the old one is still memoized, but it's not updated then.
It's not possible in Rust currently to check whether the (content of the) callback has changed with the `Fn` trait, which would make this otherwise possible.
"
        );
    };

    /// Create a new `Memoize` view.
    pub fn new(data: D, child_cb: F) -> Self {
        #[allow(clippy::let_unit_value)]
        let _ = Self::ASSERT_CONTEXTLESS_FN;
        Memoize { data, child_cb }
    }
}

impl<State, Action, Context, Data, V, ViewFn> View<State, Action, Context> for Memoize<Data, ViewFn>
where
    Context: ViewPathTracker,
    Data: PartialEq + 'static,
    V: View<State, Action, Context>,
    ViewFn: Fn(&Data) -> V + 'static,
{
    type ViewState = MemoizeState<State, Action, Context, V>;

    type Element = V::Element;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let view = (self.child_cb)(&self.data);
        let (element, view_state) = view.build(ctx);
        let memoize_state = MemoizeState {
            view,
            view_state,
            dirty: false,
        };
        (element, memoize_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if core::mem::take(&mut view_state.dirty) || prev.data != self.data {
            let view = (self.child_cb)(&self.data);
            let el = view.rebuild(&view_state.view, &mut view_state.view_state, ctx, element);
            view_state.view = view;
            el
        } else {
            element
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let message_result =
            view_state
                .view
                .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        view_state
            .view
            .teardown(&mut view_state.view_state, ctx, element);
    }
}

/// Memoize the view, until the `data` changes (in which case `view` is called again)
pub fn memoize<State, Action, Context, Data, V, ViewFn>(
    data: Data,
    view: ViewFn,
) -> Memoize<Data, ViewFn>
where
    Data: PartialEq + 'static,
    ViewFn: Fn(&Data) -> V + 'static,
    V: View<State, Action, Context>,
    Context: ViewPathTracker,
{
    Memoize::new(data, view)
}
