use crate::{AsyncCtx, DynMessage, MessageResult, NoElement, View, ViewPathTracker};

/// Start an async task from a view.
///
/// `launch` will be called to start the task.
/// If `launch` sends a message through the Proxy
/// The token returned from `launch` will be [dropped](Drop) when this view
/// is no longer in the tree.
pub fn run_async<F, Token, M, State, Action, Context>(launch: F, on_message: M) -> RunAsync<F, M>
where
    Context: AsyncCtx,
    F: Fn(Context::Proxy) -> Token + 'static,
    M: Fn(&mut State, &mut Token, DynMessage) -> MessageResult<Action> + 'static,
{
    const {
        assert!(
            std::mem::size_of::<F>() == 0,
            "Using a capturing closure in `run_async` may not have the behaviour you want.\n\
            To ignore this warning, use `run_async_raw`."
        );
        assert!(
            std::mem::size_of::<M>() == 0,
            "Captured variables in the `on_message` passed to `run_async` might not be up-to-date.\n\
            Use the provided State reference to access needed data"
        );
    };
    RunAsync { launch, on_message }
}

/// Start an async task without validating that `launch` and `on_message` don't
/// capture any variables.
// TODO: Better docs
pub fn run_async_raw<F, Token, M, State, Action, Context>(
    launch: F,
    on_message: M,
) -> RunAsync<F, M>
where
    Context: AsyncCtx,
    F: Fn(Context::Proxy) -> Token + 'static,
    M: Fn(&mut State, &mut Token, DynMessage) -> MessageResult<Action> + 'static,
{
    RunAsync { launch, on_message }
}

/// The view for [`run_async`].
pub struct RunAsync<F, M> {
    pub(crate) launch: F,
    pub(crate) on_message: M,
}

impl<F, Token, M, State, Action, Context> View<State, Action, Context> for RunAsync<F, M>
where
    Context: ViewPathTracker + AsyncCtx,
    F: Fn(Context::Proxy) -> Token + 'static,
    M: Fn(&mut State, &mut Token, DynMessage) -> MessageResult<Action> + 'static,
{
    type Element = NoElement;

    type ViewState = Option<Token>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let token = (self.launch)(ctx.proxy());
        (NoElement, Some(token))
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut Context,
        _: crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        // The task doesn't need to know about a rebuild
    }

    fn teardown(
        &self,
        state: &mut Self::ViewState,
        _: &mut Context,
        _: crate::Mut<'_, Self::Element>,
    ) {
        // The state will *probably* be dropped immediately after `teardown` is called anyway.
        drop(state.take().expect("Only teardown once"));
    }

    fn message(
        &self,
        token: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in `RunAsync::message`, as it has no children"
        );
        (self.on_message)(
            app_state,
            token.as_mut().expect("Can't get a message after teardown"),
            message,
        )
    }
}
