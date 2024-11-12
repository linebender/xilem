// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

/// A view that wraps a child view and modifies the state that callbacks have access to.
#[derive(Debug)]
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Adapt<
    ParentState,
    ParentAction,
    ChildState,
    ChildAction,
    Context,
    ChildView,
    Message,
    ProxyFn = fn(
        &mut ParentState,
        AdaptThunk<'_, ChildState, ChildAction, Context, ChildView, Message>,
    ) -> MessageResult<ParentAction>,
> {
    proxy_fn: ProxyFn,
    child: ChildView,
    phantom: PhantomData<
        fn() -> (
            ParentState,
            ParentAction,
            ChildState,
            ChildAction,
            Context,
            Message,
        ),
    >,
}

/// A "thunk" which dispatches an message to an adapt node's child.
///
/// The closure passed to [`Adapt`] should call this thunk with the child's
/// app state.
#[derive(Debug)]
pub struct AdaptThunk<'a, ChildState, ChildAction, Context, ChildView, Message>
where
    Context: ViewPathTracker,
    ChildView: View<ChildState, ChildAction, Context, Message>,
{
    child: &'a ChildView,
    view_state: &'a mut ChildView::ViewState,
    id_path: &'a [ViewId],
    message: Message,
}

/// A view that wraps a child view and modifies the state that callbacks have access to.
///
/// # Examples
///
/// Suppose you have an outer type that looks like
///
/// ```ignore
/// struct TodoList {
///     todos: Vec<Todo>
/// }
/// ```
///
/// and an inner type/view that looks like
///
/// ```ignore
/// struct Todo {
///     label: String
/// }
///
/// struct TodoView {
///     label: String
/// }
///
/// enum TodoAction {
///     Delete
/// }
///
/// impl<ViewCtx> View<Todo, TodoAction, ViewCtx> for TodoView {
///     // ...
/// }
/// ```
///
/// then your top-level action (`()`) and state type (`TodoList`) don't match `TodoView`'s.
/// You can use the `Adapt` view to mediate between them:
///
/// ```ignore
/// state
///     .todos
///     .enumerate()
///     .map(|(idx, todo)| adapt(
///         TodoView { label: todo.label },
///         |data: &mut AppState, thunk| {
///             thunk.call(&mut data.todos[idx]).map(|action| match action {
///                 TodoAction::Delete => data.todos.remove(idx),
///             })
///         })
///     )
/// ```
pub fn adapt<
    ParentState,
    ParentAction,
    ChildState,
    ChildAction,
    Context,
    ChildView,
    Message,
    ProxyFn,
>(
    child: ChildView,
    proxy_fn: ProxyFn,
) -> Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, ChildView, Message, ProxyFn>
where
    ChildState: 'static,
    ChildAction: 'static,
    ParentState: 'static,
    ParentAction: 'static,
    Context: ViewPathTracker + 'static,
    ChildView: View<ChildState, ChildAction, Context, Message>,
    ProxyFn: Fn(
            &mut ParentState,
            AdaptThunk<'_, ChildState, ChildAction, Context, ChildView, Message>,
        ) -> MessageResult<ParentAction, Message>
        + 'static,
{
    Adapt {
        proxy_fn,
        child,
        phantom: Default::default(),
    }
}

impl<'a, ChildState, ChildAction, Context, ChildView, Message>
    AdaptThunk<'a, ChildState, ChildAction, Context, ChildView, Message>
where
    Context: ViewPathTracker,
    ChildView: View<ChildState, ChildAction, Context, Message>,
{
    /// Proxies messages from the parent [`View<ParentState, ParentAction, Context>`] to the child [`View<ChildState, ChildAction, Context>`]
    ///
    /// When the `Action` types differ the `MessageResult` returned by this method has to be converted to [`MessageResult<ParentAction>`]
    pub fn call(self, app_state: &mut ChildState) -> MessageResult<ChildAction, Message> {
        self.child
            .message(self.view_state, self.id_path, self.message, app_state)
    }
}

impl<ParentState, ParentAction, ChildState, ChildAction, Context, Message, V, F> ViewMarker
    for Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, V, Message, F>
{
}
impl<ParentState, ParentAction, ChildState, ChildAction, Context, Message, V, F>
    View<ParentState, ParentAction, Context, Message>
    for Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, V, Message, F>
where
    ChildState: 'static,
    ChildAction: 'static,
    ParentState: 'static,
    ParentAction: 'static,
    Message: 'static,
    Context: ViewPathTracker + 'static,
    V: View<ChildState, ChildAction, Context, Message>,
    F: Fn(
            &mut ParentState,
            AdaptThunk<'_, ChildState, ChildAction, Context, V, Message>,
        ) -> MessageResult<ParentAction, Message>
        + 'static,
{
    type ViewState = V::ViewState;

    type Element = V::Element;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.rebuild(&prev.child, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut ParentState,
    ) -> MessageResult<ParentAction, Message> {
        let thunk = AdaptThunk {
            child: &self.child,
            view_state,
            id_path,
            message,
        };
        (self.proxy_fn)(app_state, thunk)
    }
}
