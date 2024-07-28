// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{thread, time};

use xilem::{
    core::{fork, MessageProxy},
    tokio::sync::mpsc,
    view::{async_worker, label},
    EventLoop, WidgetView, Xilem,
};

// TODO: Figure out type parameter F for returned AsyncWorker.
//
// use xilem::{core::Message, view::AsyncWorker};
//
// fn async_receiver<Handler, Msg, State, Action, Ftr>(
//     receiver: Option<mpsc::Receiver<Msg>>,
//     on_event: Handler,
// ) -> AsyncWorker<_, Handler, Msg>
// where
//     Msg: Message + 'static,
//     Handler: Fn(&mut State, Msg) -> Action + 'static,
// {
//     async_worker(
//         move |proxy: MessageProxy<Msg>| async move {
//             match receiver {
//                 Some(mut rx) => {
//                     while let Some(msg) = rx.recv().await {
//                         if proxy.message(msg).is_err() {
//                             break;
//                         }
//                     }
//                 }
//                 None => unreachable!(),
//             }
//         },
//         on_event,
//     )
// }

struct AppData {
    receiver: Option<mpsc::Receiver<i32>>,
    number: i32,
}

fn app_logic(data: &mut AppData) -> impl WidgetView<AppData> {
    let rx = data.receiver.take();
    fork(
        label(format!("Number: {}", &data.number)),
        // TODO: Finish async_receiver implementation above,
        // and use it here instead of async_worker.
        //
        // async_receiver(rx, |data: &mut AppData, msg: i32| {
        //     data.number = msg;
        // })
        async_worker(
            move |proxy: MessageProxy<i32>| async move {
                if let Some(mut rx) = rx {
                    while let Some(msg) = rx.recv().await {
                        if proxy.message(msg).is_err() {
                            break;
                        }
                    }
                }
            },
            |data: &mut AppData, msg: i32| {
                data.number = msg;
            },
        ),
    )
}

fn data_thread(sender: mpsc::Sender<i32>) {
    let mut number = 0;
    while let Ok(()) = sender.blocking_send(number) {
        number += 1;
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn main() {
    let (tx, rx) = mpsc::channel(1);
    let data = AppData {
        number: 0,
        receiver: Some(rx),
    };
    thread::spawn(move || data_thread(tx));
    Xilem::new(data, app_logic)
        .run_windowed(EventLoop::with_user_event(), "Data Thread".into())
        .unwrap();
}
