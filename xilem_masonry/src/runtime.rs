//! A simple shim for different async runtime backend.

#[cfg(all(feature = "tokio-rt", feature = "smol-rt"))]
compile_error!("Only one of `tokio-rt` or `smol-rt` can be enabled.");

#[cfg(not(any(feature = "tokio-rt", feature = "smol-rt")))]
compile_error!("One of `tokio-rt` or `smol-rt` must be enabled.");

#[cfg(feature = "tokio-rt")]
mod tokio_rt {
    use std::sync::Arc;

    use tokio::runtime::Runtime;
    use tokio::task::JoinHandle;

    pub use tokio::sync::mpsc::{
        UnboundedReceiver as Receiver, UnboundedSender as Sender, unbounded_channel as channel,
    };

    /// Wrapper around [`tokio::task::JoinHandle`].
    pub struct Handle(JoinHandle<()>);

    impl Handle {
        /// See [`tokio::task::JoinHandle::abort`].
        pub fn abort(&mut self) {
            self.0.abort();
        }
    }

    /// Wrapper around [`tokio::runtime::Runtime`].
    pub struct Executor(Arc<Runtime>);

    impl Executor {
        /// Create a new executor.
        pub fn new() -> Self {
            Self(Runtime::new().unwrap().into())
        }

        /// See [`tokio::runtime::Runtime::spawn`].
        pub fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) -> Handle {
            Handle(self.0.spawn(fut))
        }
    }

    impl From<Arc<Runtime>> for Executor {
        fn from(value: Arc<Runtime>) -> Self {
            Self(value)
        }
    }

    impl Default for Executor {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "smol-rt")]
mod smol_rt {
    use smol::{Task, channel, spawn};

    /// Wrapper around [`smol::Task`].
    pub struct Handle(Option<Task<()>>);

    impl Handle {
        /// Abort the current task.
        pub fn abort(&mut self) {
            drop(self.0.take());
        }
    }

    /// An unit struct represting `smol`'s async executor.
    pub struct Executor;

    impl Executor {
        /// Create a bew executor,
        pub fn new() -> Self {
            Self
        }

        /// Spawn a new future.
        pub fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) -> Handle {
            Handle(Some(spawn(fut)))
        }
    }

    impl Default for Executor {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Wrapper around [`smol::channel::Sender`].
    #[derive(Clone)]
    pub struct Sender<T>(channel::Sender<T>);

    /// Wrapper around [`smol::channel::Receiver`].
    pub struct Receiver<T>(channel::Receiver<T>);

    /// Wrapper around [`smol::channel::SendError`].
    pub type SendError<T> = channel::SendError<T>;

    impl<T> Sender<T> {
        /// Send a message to the receiver.
        pub fn send(&self, value: T) -> Result<(), SendError<T>> {
            self.0.force_send(value).map(drop)
        }
    }

    impl<T> Receiver<T> {
        /// Receive message from the sender.
        pub async fn recv(&mut self) -> Option<T> {
            self.0.recv().await.ok()
        }
    }

    /// Wrapper around [`smol::channel::unbounded`].
    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let (sender, receiver) = channel::unbounded();
        (Sender(sender), Receiver(receiver))
    }
}

#[cfg(feature = "tokio-rt")]
pub use tokio_rt::*;

#[cfg(feature = "smol-rt")]
pub use smol_rt::*;
