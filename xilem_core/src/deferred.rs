use alloc::vec::Vec;

use crate::{element::NoElement, DynMessage, View, ViewId, ViewPathTracker};

pub trait AsyncCtx {
    type Proxy;

    fn proxy(&mut self) -> Self::Proxy;
}

pub trait Proxy {
    fn send_message(&mut self, path: &[ViewId], message: DynMessage);
}

pub struct ChannelView {
    path: Vec<ViewId>,
}

impl<State, Action, Context> View<State, Action, Context> for ChannelView
where
    Context: ViewPathTracker + AsyncCtx,
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
