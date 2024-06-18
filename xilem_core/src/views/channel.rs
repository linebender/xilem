use std::sync::{mpsc::Receiver, Mutex};

use alloc::sync::Arc;

use crate::{deferred::AsyncCtx, DynMessage, NoElement, View, ViewId, ViewPathTracker};

pub struct ChannelView<T> {
    path: Vec<ViewId>,
    channel: Arc<Mutex<Receiver<T>>>,
}

impl<T: 'static, State, Action, Context> View<State, Action, Context> for ChannelView<T>
where
    Context: AsyncCtx,
{
    type Element = NoElement;

    type ViewState = ();

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
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        todo!()
    }
}
