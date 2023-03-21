use std::panic;

mod state;

use state::{AppState, Filter, Todo};

use wasm_bindgen::{prelude::*, JsValue};
use xilem_html::{
    elements as el, get_element_by_id, text, Adapt, App, MessageResult, View, ViewExt, ViewMarker,
};

// All of these actions arise from within a `Todo`, but we need access to the full state to reduce
// them.
enum TodoAction {
    SetEditing(u64),
    CancelEditing,
    Destroy(u64),
}

fn todo_item(todo: &mut Todo, editing: bool) -> impl View<Todo, TodoAction> + ViewMarker {
    let mut class = String::new();
    if todo.completed {
        class.push_str(" completed");
    }
    if editing {
        class.push_str(" editing");
    }
    let mut input = el::input(())
        .attr("class", "toggle")
        .attr("type", "checkbox");
    if todo.completed {
        input.set_attr("checked", "checked");
    };

    el::li((
        el::div((
            input.on_click(|state: &mut Todo, _| {
                state.completed = !state.completed;
                MessageResult::RequestRebuild
            }),
            el::label(text(todo.title.clone())).on_dblclick(|state: &mut Todo, _| {
                MessageResult::Action(TodoAction::SetEditing(state.id))
            }),
            el::button(())
                .attr("class", "destroy")
                .on_click(|state: &mut Todo, _| {
                    MessageResult::Action(TodoAction::Destroy(state.id))
                }),
        ))
        .attr("class", "view"),
        el::input(())
            .attr("value", todo.title_editing.clone())
            .attr("class", "edit")
            .on_keydown(|state: &mut Todo, evt| {
                let key = evt.key();
                if key == "Enter" {
                    state.save_editing();
                    MessageResult::Action(TodoAction::CancelEditing)
                } else if key == "Escape" {
                    MessageResult::Action(TodoAction::CancelEditing)
                } else {
                    MessageResult::Nop
                }
            })
            .on_input(|state: &mut Todo, evt| {
                state.title_editing.clear();
                state.title_editing.push_str(&evt.target().value());
                evt.prevent_default();
                MessageResult::Nop
            }),
    ))
    .attr("class", class)
}

fn footer_view(state: &mut AppState) -> impl View<AppState> + ViewMarker {
    let item_str = if state.todos.len() == 1 {
        "item"
    } else {
        "items"
    };

    let clear_button = (state.todos.iter().filter(|todo| todo.completed).count() > 0).then(|| {
        el::button(text("Clear completed"))
            .attr("class", "clear-completed")
            .on_click(|state: &mut AppState, _| {
                state.todos.retain(|todo| !todo.completed);
                MessageResult::RequestRebuild
            })
    });

    let filter_class = |filter| {
        if state.filter == filter {
            "selected"
        } else {
            ""
        }
    };

    el::footer((
        el::span((
            el::strong(text(state.todos.len().to_string())),
            text(format!(" {} left", item_str)),
        ))
        .attr("class", "todo-count"),
        el::ul((
            el::li(
                el::a(text("All"))
                    .attr("href", "#/")
                    .attr("class", filter_class(Filter::All))
                    .on_click(|state: &mut AppState, _| {
                        state.filter = Filter::All;
                        MessageResult::RequestRebuild
                    }),
            ),
            text(" "),
            el::li(
                el::a(text("Active"))
                    .attr("href", "#/active")
                    .attr("class", filter_class(Filter::Active))
                    .on_click(|state: &mut AppState, _| {
                        state.filter = Filter::Active;
                        MessageResult::RequestRebuild
                    }),
            ),
            text(" "),
            el::li(
                el::a(text("Completed"))
                    .attr("href", "#/completed")
                    .attr("class", filter_class(Filter::Completed))
                    .on_click(|state: &mut AppState, _| {
                        state.filter = Filter::Completed;
                        MessageResult::RequestRebuild
                    }),
            ),
        ))
        .attr("class", "filters"),
        clear_button,
    ))
    .attr("class", "footer")
}

fn main_view(state: &mut AppState) -> impl View<AppState> + ViewMarker {
    let editing_id = state.editing_id;
    let todos: Vec<_> = state
        .visible_todos()
        .map(|(idx, todo)| {
            Adapt::new(
                move |data: &mut AppState, thunk| {
                    if let MessageResult::Action(action) = thunk.call(&mut data.todos[idx]) {
                        match action {
                            TodoAction::SetEditing(id) => data.start_editing(id),
                            TodoAction::CancelEditing => data.editing_id = None,
                            TodoAction::Destroy(id) => data.todos.retain(|todo| todo.id != id),
                        }
                    }
                    MessageResult::Nop
                },
                todo_item(todo, editing_id == Some(todo.id)),
            )
        })
        .collect();
    el::section((
        el::input(())
            .attr("id", "toggle-all")
            .attr("class", "toggle-all")
            .attr("type", "checkbox")
            .attr("checked", "true"),
        el::label(()).attr("for", "toggle-all"),
        el::ul(todos).attr("class", "todo-list"),
    ))
    .attr("class", "main")
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    log::debug!("render: {state:?}");
    let main = (!state.todos.is_empty()).then(|| main_view(state));
    let footer = (!state.todos.is_empty()).then(|| footer_view(state));
    el::div((
        el::header((
            el::h1(text("TODOs")),
            el::input(())
                .attr("class", "new-todo")
                .attr("placeholder", "What needs to be done?")
                .attr("value", state.new_todo.clone())
                .attr("autofocus", "true")
                .on_keydown(|state: &mut AppState, evt| {
                    if evt.key() == "Enter" {
                        state.create_todo();
                    }
                    MessageResult::RequestRebuild
                })
                .on_input(|state: &mut AppState, evt| {
                    state.update_new_todo(&evt.target().value());
                    evt.prevent_default();
                    MessageResult::RequestRebuild
                }),
        ))
        .attr("class", "header"),
        main,
        footer,
    ))
}

// Called by our JS entry point to run the example
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).unwrap();
    App::new(AppState::default(), app_logic).run(&get_element_by_id("todoapp"));

    Ok(())
}
