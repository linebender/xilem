// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A to-do-list app, loosely inspired by todomvc.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]
#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use winit::error::EventLoopError;
use xilem::view::{Axis, FlexSpacer, button, checkbox, flex, textbox};
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

fn app_logic(task_list: &mut TaskList) -> impl WidgetView<TaskList> + use<> {
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

    flex((
        FlexSpacer::Fixed(40.), // HACK: Spacer for Androird
        first_line,
        tasks,
    ))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = TaskList {
        // Add a placeholder task for Android, whilst the
        next_task: "My Next Task".into(),
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
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
