// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt::{Debug, Display};
use core::marker::PhantomData;

use crate::{DynMessage, Message, NoElement, View, ViewId, ViewPathTracker};

/// A `Context` for a [`View`] implementation which supports
/// asynchronous message reporting.
pub trait AsyncCtx<Message = DynMessage>: ViewPathTracker {
    /// Get a [`RawProxy`] for this context.
    // TODO: Maybe store the current path within this Proxy?
    fn proxy(&mut self) -> Arc<dyn RawProxy<Message>>;
}

/// A handle to a Xilem driver which can be used to queue a message for a View.
///
/// These messages are [`crate::DynMessage`]s, which are sent to a view at
/// a specific path.
///
/// This can be used for asynchronous event handling.
/// For example, to get the result of a `Future` or a channel into
/// the view, which then will ultimately.
///
/// In the Xilem crate, this will wrap an `EventLoopProxy` from Winit.
///
/// ## Lifetimes
///
/// It is valid for a [`RawProxy`] to outlive the [`View`] it is associated with.
pub trait RawProxy<Message = DynMessage>: Send + Sync + 'static {
    /// Send a `message` to the view at `path` in this driver.
    ///
    /// Note that it is only valid to send messages to views which expect
    /// them, of the type they expect.
    /// It is expected for [`View`]s to panic otherwise, and the routing
    /// will prefer to send stable.
    ///
    /// # Errors
    ///
    /// This method may error if the driver is no longer running, and in any other
    /// cases directly documented on the context which was used to create this proxy.
    /// It may also fail silently.
    // TODO: Do we want/need a way to asynchronously report errors back to the caller?
    //
    // e.g. an `Option<Arc<dyn FnMut(ProxyError, ProxyMessageId?)>>`?
    fn send_message(&self, path: Arc<[ViewId]>, message: Message) -> Result<(), ProxyError>;
    /// Get the debug formatter for this proxy type.
    fn dyn_debug(&self) -> &dyn Debug;
}

impl<Message: 'static> Debug for dyn RawProxy<Message> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.dyn_debug().fmt(f)
    }
}

/// A way to send a message of an expected type to a specific view.
#[derive(Debug)]
pub struct MessageProxy<M: Message> {
    proxy: Arc<dyn RawProxy<DynMessage>>,
    path: Arc<[ViewId]>,
    message: PhantomData<fn(M)>,
}

impl<M: Message> Clone for MessageProxy<M> {
    fn clone(&self) -> Self {
        Self {
            proxy: self.proxy.clone(),
            path: self.path.clone(),
            message: PhantomData,
        }
    }
}

impl<M: Message> MessageProxy<M> {
    /// Create a new `MessageProxy`
    pub fn new(proxy: Arc<dyn RawProxy<DynMessage>>, path: Arc<[ViewId]>) -> Self {
        Self {
            proxy,
            path,
            message: PhantomData,
        }
    }

    /// Send `message` to the `View` which created this `MessageProxy`
    pub fn message(&self, message: M) -> Result<(), ProxyError> {
        self.proxy
            .send_message(self.path.clone(), Box::new(message))
    }
}

/// A [`View`] which has no element type.
pub trait PhantomView<State, Action, Context, Message = DynMessage>:
    View<State, Action, Context, Message, Element = NoElement>
where
    Context: ViewPathTracker,
{
}

impl<State, Action, Context, Message, V> PhantomView<State, Action, Context, Message> for V
where
    V: View<State, Action, Context, Message, Element = NoElement>,
    Context: ViewPathTracker,
{
}

/// The potential error conditions from a [`RawProxy`] sending a message
#[derive(Debug)]
pub enum ProxyError {
    /// The underlying driver (such as an event loop) is no longer running.
    ///
    /// TODO: Should this also support a source message?
    DriverFinished(DynMessage),
    /// The [`View`] the message was being routed to is no longer in the view tree.
    ///
    /// This likely requires async error handling to happen.
    ViewExpired(DynMessage, Arc<[ViewId]>),
    /// An error specific to the driver being used.
    Other(Box<dyn core::error::Error + Send>),
}

// Is it fine to use thiserror in this crate?
impl Display for ProxyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::DriverFinished(_) => f.write_fmt(format_args!("the driver finished")),
            Self::ViewExpired(_, _) => {
                f.write_fmt(format_args!("the corresponding view is no longer present"))
            }
            Self::Other(inner) => Display::fmt(inner, f),
        }
    }
}

impl core::error::Error for ProxyError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Other(inner) => inner.source(),
            _ => None,
        }
    }
}
