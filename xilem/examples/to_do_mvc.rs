// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use xilem::view::{button, checkbox, flex, textbox, Axis};
use xilem::{EventLoop, EventLoopBuilder, WidgetView, Xilem};

struct Task {
    description: String,
    done: bool,
}

struct TaskList {
    next_task: String,
    tasks: Vec<Task>,
}

impl TaskList {
    fn add_task(&mut self) {
        if self.next_task.is_empty() {
            return;
        }
        self.tasks.push(Task {
            description: std::mem::take(&mut self.next_task),
            done: false,
        });
    }
}

fn app_logic(task_list: &mut TaskList) -> impl WidgetView<TaskList> {
    let input_box = textbox(
        task_list.next_task.clone(),
        |task_list: &mut TaskList, new_value| {
            task_list.next_task = new_value;
        },
    )
    .on_enter(|task_list: &mut TaskList, _| {
        task_list.add_task();
    });
    let first_line = flex((
        input_box,
        button("Add task".to_string(), |task_list: &mut TaskList| {
            task_list.add_task();
        }),
    ))
    .direction(Axis::Vertical);

    let tasks = task_list
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let checkbox = checkbox(
                task.description.clone(),
                task.done,
                move |data: &mut TaskList, checked| {
                    data.tasks[i].done = checked;
                },
            );
            let delete_button = button("Delete", move |data: &mut TaskList| {
                data.tasks.remove(i);
            });
            flex((checkbox, delete_button)).direction(Axis::Horizontal)
        })
        .collect::<Vec<_>>();

    #[cfg(not(target_os = "android"))]
    return flex((first_line, tasks));
    #[cfg(target_os = "android")]
    return flex((
        (xilem::view::FlexSpacer::Fixed(40.)),
        first_line,
        tasks,
        xilem::view::FlexSpacer::Fixed(30.),
    ));
}

fn run(event_loop: EventLoopBuilder) {
    let data = TaskList {
        next_task: String::new(),
        tasks: vec![
            Task {
                description: "Buy milk".into(),
                done: false,
            },
            Task {
                description: "Buy eggs".into(),
                done: true,
            },
            Task {
                description: "Buy bread".into(),
                done: false,
            },
        ],
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed(event_loop, "First Example".into())
        .unwrap();
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() {
    run(EventLoop::with_user_event());
}

// Boilerplate code for android: Identical across all applications

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(target_os = "android")]
use xilem_core::{fork, run_once};

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
// We believe that there are no other declarations using this name in the compiled objects here
#[allow(unsafe_code)]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop);
}

// TODO: This is a hack because of how we handle our examples in Cargo.toml
// Ideally, we change Cargo to be more sensible here?
#[cfg(target_os = "android")]
#[allow(dead_code)]
fn main() {
    unreachable!()
}
