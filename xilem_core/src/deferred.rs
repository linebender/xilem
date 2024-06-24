use core::fmt::Display;

use alloc::sync::Arc;

use crate::{DynMessage, ViewId, ViewPathTracker};

/// A `Context` for a [`View`](crate::View) implementation which supports
/// asynchronous message reporting.
pub trait AsyncCtx: ViewPathTracker {
    /// The [`Proxy`] type used to access this view.
    type Proxy: Proxy;

    /// Get a [`Proxy`] for this
    fn proxy(&mut self) -> Self::Proxy;
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
/// It is valid for a [`Proxy`] to outlive the [`View`](crate::View) it is associated with.
pub trait Proxy: Send + 'static {
    /// Send a `message` to the view at `path` in this driver.
    ///
    /// Note that it is only valid to send messages to views which expect
    /// them, of the type they expect.
    /// It is expected for [`View`](crate::View)s to panic otherwise, and the routing
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
    fn send_message(&mut self, path: Arc<[ViewId]>, message: DynMessage) -> Result<(), ProxyError>;
}

/// The potential error conditions from a [`Proxy`] sending a message
#[derive(Debug)]
pub enum ProxyError {
    /// The underlying driver (such as an event loop) is no longer running.
    DriverFinished(DynMessage),
    /// The [`View`](crate::View) the message was being routed to is no longer in the view tree.
    ViewExpired(DynMessage, Arc<[ViewId]>),
    /// Any other error condition.
    ///
    /// This variant is equivalent to [ProxyError::Other], but is
    /// supported when the std feature is not enabled.
    ///
    /// This will be removed if [`std::error::Error`] is moved to `core`.
    /// You should prefer `Other` if using Xilem Core with `std`.
    CoreOther(Box<dyn core::fmt::Debug + Send>),
    #[cfg(feature = "std")]
    Other(Box<dyn std::error::Error + Send>),
}

// Is it fine to use thiserror in this crate?
impl Display for ProxyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            ProxyError::DriverFinished(_) => f.write_fmt(format_args!("the driver finished")),
            ProxyError::ViewExpired(_, _) => {
                f.write_fmt(format_args!("the corresponding view is no longer present"))
            }
            ProxyError::CoreOther(inner) => {
                // TODO: Do we want to use a Display impl here, and so forward here
                f.write_fmt(format_args!("some other error occurred: {inner:?}"))
            }
            // This is the equivalent to
            ProxyError::Other(inner) => inner.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProxyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProxyError::Other(inner) => inner.source(),
            _ => None,
        }
    }
}
