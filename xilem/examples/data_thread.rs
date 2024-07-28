// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{sync::mpsc, thread, time};

use xilem::{
    core::{fork, MessageProxy},
    view::{label, message_handler},
    EventLoop, WidgetView, Xilem,
};

struct AppData {
    proxy_sender: mpsc::SyncSender<MessageProxy<i32>>,
    number: i32,
}

fn app_logic(data: &mut AppData) -> impl WidgetView<AppData> {
    fork(
        label(format!("Number: {}", &data.number)),
        message_handler(
            |data: &mut AppData, proxy: MessageProxy<i32>| {
                data.proxy_sender.send(proxy).unwrap();
            },
            |data: &mut AppData, msg: i32| {
                data.number = msg;
            },
        ),
    )
}

fn data_thread(proxy_receiver: mpsc::Receiver<MessageProxy<i32>>) {
    if let Ok(proxy) = proxy_receiver.recv() {
        let mut number = 0;
        while let Ok(()) = proxy.message(number) {
            number += 1;
            thread::sleep(time::Duration::from_secs(1));
        }
    }
}

fn main() {
    let (proxy_sender, proxy_receiver) = mpsc::sync_channel(1);
    let data = AppData {
        proxy_sender,
        number: 0,
    };
    thread::spawn(move || data_thread(proxy_receiver));
    Xilem::new(data, app_logic)
        .run_windowed(EventLoop::with_user_event(), "Centered Flex".into())
        .unwrap();
}
