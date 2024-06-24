use crate::{AsyncCtx, DynMessage, MessageResult, NoElement, View, ViewPathTracker};

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
        )
    };
    RunAsync { launch, on_message }
}

pub fn run_async_raw<F, Token, M, State, Action, Context>(
    launch: F,
    on_message: M,
) -> RunAsync<F, M>
where
    Context: AsyncCtx,
    F: Fn(Context::Proxy) -> Token + 'static,
    M: Fn(&mut State, &mut Token, DynMessage) -> MessageResult<Action> + 'static,
{
    const { assert!(std::mem::size_of::<F>() == 0, "run_once does not support") };
    RunAsync { launch, on_message }
}

pub struct RunAsync<F, M> {
    pub(crate) launch: F,
    pub(crate) on_message: M,
}

pub trait AsyncToken {}

impl<F, Token, M, State, Action, Context> View<State, Action, Context> for RunAsync<F, M>
where
    Context: ViewPathTracker + AsyncCtx,
    F: Fn(Context::Proxy) -> Token + 'static,
    M: Fn(&mut State, &mut Token, DynMessage) -> MessageResult<Action> + 'static,
{
    type Element = NoElement;

    type ViewState = Token;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        todo!()
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        todo!()
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in RunOnce::message, as it has no children"
        );
        (self.on_message)(app_state, message)
    }
}
