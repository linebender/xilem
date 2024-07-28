// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, ops::Deref, sync::Arc};

use xilem_core::{
    DynMessage, Message, MessageProxy, NoElement, RawProxy, View, ViewId, ViewPathTracker,
};

use crate::ViewCtx;

/// No-element view which allows to update app-data in response to
/// asynchronous user messages.
///
/// `store_proxy` serves as a way to obtain [`MessageProxy`], which can then
/// be used to send messages to self.
/// It is given a mutable reference to the app data and a proxy, so the proxy
/// can be saved to the app data here, or sent to another thread for example.
/// Note, it is always called only once, changes to the app data won't trigger
/// `store_proxy` to rerun.
///
/// `handle_event` receives messages from the aforementioned `MessageProxy`,
/// along with a mutable reference to the app data.
pub fn message_handler<M, F, H, State, Action>(
    store_proxy: F,
    handle_event: H,
) -> MessageHandler<F, H, M>
where
    F: Fn(&mut State, MessageProxy<M>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    MessageHandler {
        store_proxy,
        handle_event,
        message: PhantomData,
    }
}

#[derive(Debug)]
struct StoreProxyMessage;

pub struct MessageHandler<F, H, M> {
    store_proxy: F,
    handle_event: H,
    message: PhantomData<fn() -> M>,
}

impl<State, Action, F, H, M> View<State, Action, ViewCtx> for MessageHandler<F, H, M>
where
    F: Fn(&mut State, MessageProxy<M>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    type Element = NoElement;
    type ViewState = (Arc<dyn RawProxy<DynMessage>>, Arc<[ViewId]>);

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();
        ctx.proxy
            .send_message(path.clone(), Box::new(StoreProxyMessage))
            .unwrap();
        (NoElement, (ctx.proxy.clone(), path.clone()))
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): xilem_core::Mut<'el, Self::Element>,
    ) -> xilem_core::Mut<'el, Self::Element> {
        // Nothing to do
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: xilem_core::Mut<'_, Self::Element>,
    ) {
        // Nothing to do
    }

    fn message(
        &self,
        (raw_proxy, path): &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in MessageHandler::message"
        );
        if message.deref().as_any().is::<StoreProxyMessage>() {
            let proxy = MessageProxy::new(raw_proxy.clone(), path.clone());
            (self.store_proxy)(app_state, proxy);
            xilem_core::MessageResult::Nop
        } else {
            let message = message.downcast::<M>().unwrap();
            xilem_core::MessageResult::Action((self.handle_event)(app_state, *message))
        }
    }
}
