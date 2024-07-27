// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewPathTracker};

/// A view which supports Memoization.
///
/// The story of Memoization in Xilem is still being worked out,
/// so the details of this view might change.
pub struct Memoize<Data, InitView, State, Action> {
    data: Data,
    init_view: InitView,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub struct MemoizeState<V, VState> {
    view: V,
    view_state: VState,
    dirty: bool,
}

impl<Data, V, InitView, State, Action> Memoize<Data, InitView, State, Action>
where
    InitView: Fn(&Data) -> V,
{
    /// Create a new `Memoize` view.
    pub fn new(data: Data, init_view: InitView) -> Self {
        const {
            assert!(
                core::mem::size_of::<InitView>() == 0,
                "
It's not possible to use function pointers or captured context in closures,
as this potentially messes up the logic of memoize or produces unwanted effects.

For example a different kind of view could be instantiated with a different callback, while the old one is still memoized, but it's not updated then.
It's not possible in Rust currently to check whether the (content of the) callback has changed with the `Fn` trait, which would make this otherwise possible.
"
        );
        };
        Memoize {
            data,
            init_view,
            phantom: PhantomData,
        }
    }
}

impl<State, Action, Context, Data, V, ViewFn, Message> View<State, Action, Context, Message>
    for Memoize<Data, ViewFn, State, Action>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker,
    Data: PartialEq + 'static,
    V: View<State, Action, Context, Message>,
    ViewFn: Fn(&Data) -> V + 'static,
{
    type ViewState = MemoizeState<V, V::ViewState>;

    type Element = V::Element;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let view = (self.init_view)(&self.data);
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
            let view = (self.init_view)(&self.data);
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
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
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
pub fn memoize<State, Action, Context, Message, Data, V, InitView>(
    data: Data,
    init_view: InitView,
) -> Memoize<Data, InitView, State, Action>
where
    Data: PartialEq + 'static,
    InitView: Fn(&Data) -> V + 'static,
    V: View<State, Action, Context, Message>,
    Context: ViewPathTracker,
{
    Memoize::new(data, init_view)
}

/// Specialized version of [`Memoize`], which doesn't take any data at all, the closure is evaluated only once and when a child view forces a rebuild
pub struct Static<InitView, State, Action> {
    init_view: InitView,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Specialized version of [`memoize`], which doesn't take any data at all, the closure is evaluated only once and when a child view forces a rebuild
pub fn static_view<State, Action, Context, Message, V, InitView>(
    init_view: InitView,
) -> Static<InitView, State, Action>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message>,
    InitView: Fn() -> V,
{
    const {
        assert!(
            core::mem::size_of::<InitView>() == 0,
                "
It's not possible to use function pointers or captured context in closures,
as this potentially messes up the logic of memoize or produces unwanted effects.

For example a different kind of view could be instantiated with a different callback, while the old one is still memoized, but it's not updated then.
It's not possible in Rust currently to check whether the (content of the) callback has changed with the `Fn` trait, which would make this otherwise possible.
"
        );
    };
    Static {
        init_view,
        phantom: PhantomData,
    }
}

impl<State, Action, Context, Message, V, InitView> View<State, Action, Context, Message>
    for Static<InitView, State, Action>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message>,
    InitView: Fn() -> V + 'static,
{
    type Element = V::Element;

    type ViewState = MemoizeState<V, V::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let view = (self.init_view)();
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
        _prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        if core::mem::take(&mut view_state.dirty) {
            let view = (self.init_view)();
            let element =
                view_state
                    .view
                    .rebuild(&view_state.view, &mut view_state.view_state, ctx, element);
            view_state.view = view;
            element
        } else {
            element
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'_, Self::Element>,
    ) {
        view_state
            .view
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Message,
        app_state: &mut State,
    ) -> crate::MessageResult<Action, Message> {
        let message_result =
            view_state
                .view
                .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}
