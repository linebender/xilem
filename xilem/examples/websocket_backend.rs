// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A simple frontend example which communicates with backend via WebSocket
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]

const LIST: &str = r#"{"action": "list"}"#;

use async_channel::{Receiver, Sender, unbounded};
use futures_util::{SinkExt, StreamExt};
use masonry::widgets::{CrossAxisAlignment, MainAxisAlignment};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::{MessageProxy, fork};
use xilem::view::{Axis, button, flex, label, task_raw};
use xilem::{EventLoop, EventLoopBuilder, WidgetView, Xilem};

struct BackendApp {
    tx: UnboundedSender<String>,
    receive: Receiver<String>,
}

impl BackendApp {
    /// Let's connect to the backend via WebSocket
    async fn connect(socket_path: &str, module: &str) -> anyhow::Result<Self> {
        let stream = tokio::net::TcpStream::connect(socket_path).await?;
        let url = format!("ws://localhost/backend/{}", module);

        let request = tungstenite::handshake::client::Request::builder()
            .uri(&url)
            .header("Host", "localhost")
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Key", "random_key==")
            .header("Sec-WebSocket-Version", "13")
            .body(())?;

        let (ws_stream, _) = tokio_tungstenite::client_async(request, stream).await?;

        let (tx_to_backend, rx_from_gui) = tokio::sync::mpsc::unbounded_channel();
        let (tx_to_gui, rx_from_backend) = unbounded();

        tokio::spawn(Self::handle_msg(ws_stream, rx_from_gui, tx_to_gui));

        Ok(Self {
            tx: tx_to_backend,
            receive: rx_from_backend,
        })
    }

    /// Send message to the backend
    fn send(&self, message: &str) -> Result<(), String> {
        tracing::debug!("Message: {message}");
        self.tx.send(message.to_string()).map_err(|e| e.to_string())
    }

    async fn handle_msg(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        mut task: UnboundedReceiver<String>,
        backend_answer: Sender<String>,
    ) {
        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                Some(msg) = read.next() => {
                    match msg {
                        Ok(tungstenite::Message::Text(text)) => {
                            tracing::debug!("Text message: {text}");
                            if let Err(err) = backend_answer.send(text).await {
                                tracing::error!("{err}");
                            };
                        }
                        Ok(tungstenite::Message::Ping(_)) => {
                            if let Err(err) = write.send(tungstenite::Message::Pong(vec![])).await {
                                tracing::error!("{err}");
                            };
                        }
                        _ => {}
                    }
                },
                msg = task.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(err) = write.send(tungstenite::Message::Text(msg)).await{
                                tracing::error!("{err}");
                            };
                        },
                        None => {
                            tracing::debug!("None result from the backend");
                        }
                    }
                }
            }
        }
    }
}

struct FrontendApp {
    socket: BackendApp,
    msg: String,
}

impl FrontendApp {
    async fn new() -> Self {
        FrontendApp {
            socket: BackendApp::connect("localhost:9001", "db").await.unwrap(),
            msg: String::new(),
        }
    }

    fn list(&mut self) {
        self.socket.send(LIST).unwrap()
    }
}

async fn backend(proxy: MessageProxy<String>, rx: Receiver<String>) {
    if let Some(string) = rx.recv().await.ok() {
        drop(proxy.message(string));
    }
}

fn app_logic(data: &mut FrontendApp) -> impl WidgetView<FrontendApp> + use<> {
    let rx = data.socket.receive.clone();
    fork(
        flex((label(data.msg.as_ref()), button("List", FrontendApp::list)))
            .direction(Axis::Vertical)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_alignment(MainAxisAlignment::Center),
        // As far as I understand we need to use task_raw insteed of task because of 'rx'
        task_raw(
            move |proxy| backend(proxy, rx.clone()),
            |state: &mut FrontendApp, msg: String| {
                state.msg = msg;
            },
        ),
    )
}

async fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = FrontendApp::new().await;

    let app = Xilem::new(data, app_logic);
    let min_window_size = LogicalSize::new(200., 200.);
    let window_attributes = Window::default_attributes()
        .with_title("FrontendApp")
        .with_resizable(true)
        .with_min_inner_size(min_window_size);
    // On iOS, winit has unsensible handling of `inner_size`
    // See https://github.com/rust-windowing/winit/issues/2308 for more details
    #[cfg(target_os = "ios")]
    let window_attributes = {
        let mut window_attributes = window_attributes; // to avoid `unused_mut`
        window_attributes.inner_size = None;
        window_attributes
    };
    app.run_windowed_in(event_loop, window_attributes)?;
    Ok(())
}

#[tokio::main]
#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
async fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event()).await
}
