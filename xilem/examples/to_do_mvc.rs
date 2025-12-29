// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A to-do-list app, loosely inspired by todomvc.

use xilem::core::Edit;
use xilem::masonry::properties::types::Length;
use xilem::masonry::theme::{DEFAULT_GAP, ZYNC_800};
use xilem::style::Style as _;
use xilem::view::{
    FlexExt, FlexSpacer, button, checkbox, flex_col, flex_row, label, text_button, text_input,
};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, EventLoopBuilder, InsertNewline, WidgetView, WindowOptions, Xilem};

struct Task {
    description: String,
    done: bool,
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Filter {
    All,
    Active,
    Completed,
}

struct TaskList {
    next_task: String,
    filter: Filter,
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

fn app_logic(task_list: &mut TaskList) -> impl WidgetView<Edit<TaskList>> + use<> {
    let header_text = label("todos").text_size(80.);
    let input_box = text_input(
        task_list.next_task.clone(),
        |task_list: &mut TaskList, new_value| {
            task_list.next_task = new_value;
        },
    )
    //.text_size(16.)
    .placeholder("What needs to be done?")
    .insert_newline(InsertNewline::OnShiftEnter)
    .on_enter(|task_list: &mut TaskList, _| {
        task_list.add_task();
    });

    let input_box2 = text_input(String::new(), |task_list: &mut TaskList, new_value| {
        task_list.next_task = new_value;
    })
    .placeholder("What needs to be done?");

    let input_line = flex_row((
        input_box.flex(1.0),
        button(
            label("Add task".to_string()).text_size(16.),
            |task_list: &mut TaskList| {
                task_list.add_task();
            },
        ),
    ));

    let tasks = task_list
        .tasks
        .iter()
        .enumerate()
        .filter_map(|(i, task)| {
            if (task_list.filter == Filter::Active && task.done)
                || (task_list.filter == Filter::Completed && !task.done)
            {
                None
            } else {
                let checkbox = checkbox(
                    task.description.clone(),
                    task.done,
                    move |data: &mut TaskList, checked| {
                        data.tasks[i].done = checked;
                    },
                )
                .text_size(16.);
                let delete_button = text_button("Delete", move |data: &mut TaskList| {
                    data.tasks.remove(i);
                })
                .padding(5.0);
                Some(
                    flex_row((checkbox, FlexSpacer::Flex(1.), delete_button))
                        .padding(DEFAULT_GAP.get())
                        .border(ZYNC_800, 1.0),
                )
            }
        })
        .collect::<Vec<_>>();

    let filter_tasks = |label, filter| {
        // TODO: replace with combo-buttons
        checkbox::<_, Edit<TaskList>, _>(
            label,
            task_list.filter == filter,
            move |state: &mut TaskList, _| state.filter = filter,
        )
    };
    let has_tasks = !task_list.tasks.is_empty();
    let footer = has_tasks.then(|| {
        flex_row((
            filter_tasks("All", Filter::All),
            filter_tasks("Active", Filter::Active),
            filter_tasks("Completed", Filter::Completed),
        ))
    });

    flex_col((
        header_text,
        FlexSpacer::Fixed(DEFAULT_GAP),
        input_line,
        input_box2,
        FlexSpacer::Fixed(DEFAULT_GAP),
        tasks,
        FlexSpacer::Fixed(DEFAULT_GAP),
        footer,
    ))
    .gap(Length::px(4.))
    .padding(50.0)
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = TaskList {
        // Add a placeholder task for Android, whilst the
        next_task: "My Next Task".into(),
        filter: Filter::All,
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

    let app = Xilem::new_simple(data, app_logic, WindowOptions::new("To Do MVC"));
    app.run_in(event_loop)
}

// Boilerplate code: Identical across all applications which support Android

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
