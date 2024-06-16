use core::marker::PhantomData;

use crate::{DynMessage, MessageResult, Mut, View, ViewId, ViewPathTracker};

/// A view that wraps a child view and modifies the state that callbacks have access to.
///
/// # Examples
///
/// Suppose you have an outer type that looks like
///
/// ```ignore
/// struct State {
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
/// impl View<Todo, TodoAction, ViewCtx> for TodoView {
///     // ...
/// }
/// ```
///
/// then your top-level action (`()`) and state type (`State`) don't match `TodoView`'s.
/// You can use the `Adapt` view to mediate between them:
///
/// ```ignore
/// state
///     .todos
///     .enumerate()
///     .map(|(idx, todo)| {
///         Adapt::new(
///             move |data: &mut AppState, thunk| {
///                 if let MessageResult::Action(action) = thunk.call(&mut data.todos[idx]) {
///                     match action {
///                         TodoAction::Delete => data.todos.remove(idx),
///                     }
///                 }
///                 MessageResult::Nop
///             },
///             TodoView { label: todo.label }
///         )
///     })
/// ```
pub struct Adapt<
    ParentState,
    ParentAction,
    ChildState,
    ChildAction,
    Context,
    V,
    F = fn(
        &mut ParentState,
        AdaptThunk<ChildState, ChildAction, Context, V>,
    ) -> MessageResult<ParentAction>,
> where
    Context: ViewPathTracker,
{
    f: F,
    child: V,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<fn() -> (ParentState, ParentAction, ChildState, ChildAction, Context)>,
}

/// A "thunk" which dispatches an message to an adapt node's child.
///
/// The closure passed to [`Adapt`] should call this thunk with the child's
/// app state.
pub struct AdaptThunk<'a, ChildState, ChildAction, Context, V>
where
    Context: ViewPathTracker,
    V: View<ChildState, ChildAction, Context>,
{
    child: &'a V,
    view_state: &'a mut V::ViewState,
    id_path: &'a [ViewId],
    message: DynMessage,
}

impl<ParentState, ParentAction, ChildState, ChildAction, Context, ChildView, ProxyFn>
    Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, ChildView, ProxyFn>
where
    Context: ViewPathTracker,
    ChildView: View<ChildState, ChildAction, Context>,
    ProxyFn: Fn(
        &mut ParentState,
        AdaptThunk<ChildState, ChildAction, Context, ChildView>,
    ) -> MessageResult<ParentAction>,
{
    /// Creates a new [`Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, V, F>`].
    pub fn new(f: ProxyFn, child: ChildView) -> Self {
        Adapt {
            f,
            child,
            phantom: Default::default(),
        }
    }
}

impl<'a, ChildState, ChildAction, Context, ChildView>
    AdaptThunk<'a, ChildState, ChildAction, Context, ChildView>
where
    Context: ViewPathTracker,
    ChildView: View<ChildState, ChildAction, Context>,
{
    /// Proxies messages from the parent [`View<ParentState, ParentAction, Context>`] to the child [`View<ChildState, ChildAction, Context>`]
    ///
    /// When the `Action` types differ the `MessageResult` returned by this method has to be converted to [`MessageResult<ParentAction>`]
    pub fn call(self, app_state: &mut ChildState) -> MessageResult<ChildAction> {
        self.child
            .message(self.view_state, self.id_path, self.message, app_state)
    }
}

impl<ParentState, ParentAction, ChildState, ChildAction, Context, V, F>
    View<ParentState, ParentAction, Context>
    for Adapt<ParentState, ParentAction, ChildState, ChildAction, Context, V, F>
where
    ChildState: 'static,
    ChildAction: 'static,
    ParentState: 'static,
    ParentAction: 'static,
    Context: ViewPathTracker + 'static,
    V: View<ChildState, ChildAction, Context>,
    F: Fn(
            &mut ParentState,
            AdaptThunk<ChildState, ChildAction, Context, V>,
        ) -> MessageResult<ParentAction>
        + 'static,
{
    type ViewState = V::ViewState;

    type Element = V::Element;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        self.child.rebuild(&prev.child, view_state, ctx, element)
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
        message: crate::DynMessage,
        app_state: &mut ParentState,
    ) -> MessageResult<ParentAction> {
        let thunk = AdaptThunk {
            child: &self.child,
            view_state,
            id_path,
            message,
        };
        (self.f)(app_state, thunk)
    }
}
