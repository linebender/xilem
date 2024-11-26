// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;

use anyhow::anyhow;
use async_stream::stream;
use futures::{channel::oneshot, select, FutureExt, Stream};
use gloo_timers::future::TimeoutFuture;
use rand::Rng;

#[derive(Debug)]
pub(crate) enum StreamMessage {
    Started(AbortHandle),
    SearchResult(anyhow::Result<Option<String>>),
    Aborted,
    TimedOut,
    Finished,
}

#[derive(Default)]
pub(crate) struct MockConnection;

impl MockConnection {
    pub(crate) fn search(&self, search_term: String) -> impl Stream<Item = StreamMessage> {
        let (shutdown_signal, abort_handle) = ShutdownSignal::new();
        let mut shutdown = shutdown_signal.into_future().fuse();
        stream! {
            yield StreamMessage::Started(abort_handle);
            loop {
                let mut timeout = TimeoutFuture::new(3_000).fuse();
                let mut search_result = Box::pin(simulated_search(&search_term).fuse());
                select! {
                    () = shutdown => {
                        yield StreamMessage::Aborted;
                        break;
                    }
                    () = timeout => {
                        yield StreamMessage::TimedOut;
                        return;
                    }
                    result = search_result => {
                        let search_more = match &result {
                           Ok(Some(_)) => true,
                           Ok(None) | Err(_) => false
                        };
                        yield StreamMessage::SearchResult(result);
                        if !search_more {
                           break;
                        }
                    }
                    complete => panic!("stream completed unexpectedly"),
                }
            }
            yield StreamMessage::Finished;
        }
    }
}

async fn simulated_search(search_term: &str) -> anyhow::Result<Option<String>> {
    let mut rng = rand::thread_rng();

    let delay_ms = rng.gen_range(5..=600);
    TimeoutFuture::new(delay_ms).fuse().await;
    let random_value: u8 = rng.gen_range(0..=100);

    match random_value {
        0..=10 => Err(anyhow!("Simulated error")),
        11..=50 => Ok(None), // Nothing found
        _ => Ok(Some(format!("Result for '{search_term}'"))),
    }
}

#[derive(Debug)]
pub(crate) struct AbortHandle {
    abort_tx: oneshot::Sender<()>,
}

impl AbortHandle {
    pub(crate) fn abort(self) {
        let _ = self.abort_tx.send(());
    }
}

struct ShutdownSignal {
    shutdown_rx: oneshot::Receiver<()>,
}

impl ShutdownSignal {
    fn new() -> (Self, AbortHandle) {
        let (abort_tx, shutdown_rx) = oneshot::channel();
        (Self { shutdown_rx }, AbortHandle { abort_tx })
    }

    fn into_future(self) -> impl Future<Output = ()> {
        self.shutdown_rx.map(|_| ())
    }
}
