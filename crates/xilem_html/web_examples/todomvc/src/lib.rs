use std::panic;

mod state;

use state::{AppState, Filter, Todo};

use wasm_bindgen::{prelude::*, JsValue};
use xilem_html::{
    elements as el, events::on_click, get_element_by_id, Action, Adapt, App, MessageResult, View,
    ViewExt, ViewMarker,
};

// All of these actions arise from within a `Todo`, but we need access to the full state to reduce
// them.
enum TodoAction {
    SetEditing(u64),
    CancelEditing,
    Destroy(u64),
}

impl Action for TodoAction {}

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
            }),
            el::label(todo.title.clone())
                .on_dblclick(|state: &mut Todo, _| TodoAction::SetEditing(state.id)),
            el::button(())
                .attr("class", "destroy")
                .on_click(|state: &mut Todo, _| TodoAction::Destroy(state.id)),
        ))
        .attr("class", "view"),
        el::input(())
            .attr("value", todo.title_editing.clone())
            .attr("class", "edit")
            .on_keydown(|state: &mut Todo, evt| {
                let key = evt.key();
                if key == "Enter" {
                    state.save_editing();
                    Some(TodoAction::CancelEditing)
                } else if key == "Escape" {
                    Some(TodoAction::CancelEditing)
                } else {
                    None
                }
            })
            .on_input(|state: &mut Todo, evt| {
                state.title_editing.clear();
                state.title_editing.push_str(&evt.target().value());
                evt.prevent_default();
            })
            .on_blur(|_, _| TodoAction::CancelEditing),
    ))
    .attr("class", class)
}

fn footer_view(state: &mut AppState, should_display: bool) -> impl View<AppState> + ViewMarker {
    let item_str = if state.todos.len() == 1 {
        "item"
    } else {
        "items"
    };

    let clear_button = (state.todos.iter().filter(|todo| todo.completed).count() > 0).then(|| {
        on_click(
            el::button("Clear completed").attr("class", "clear-completed"),
            |state: &mut AppState, _| {
                state.todos.retain(|todo| !todo.completed);
            },
        )
    });

    let filter_class = |filter| {
        if state.filter == filter {
            "selected"
        } else {
            ""
        }
    };

    let mut footer = el::footer((
        el::span((
            el::strong(state.todos.len().to_string()),
            format!(" {} left", item_str),
        ))
        .attr("class", "todo-count"),
        el::ul((
            el::li(on_click(
                el::a("All")
                    .attr("href", "#/")
                    .attr("class", filter_class(Filter::All)),
                |state: &mut AppState, _| {
                    state.filter = Filter::All;
                },
            )),
            " ",
            el::li(on_click(
                el::a("Active")
                    .attr("href", "#/active")
                    .attr("class", filter_class(Filter::Active)),
                |state: &mut AppState, _| {
                    state.filter = Filter::Active;
                },
            )),
            " ",
            el::li(on_click(
                el::a("Completed")
                    .attr("href", "#/completed")
                    .attr("class", filter_class(Filter::Completed)),
                |state: &mut AppState, _| {
                    state.filter = Filter::Completed;
                },
            )),
        ))
        .attr("class", "filters"),
        clear_button,
    ))
    .attr("class", "footer");
    if !should_display {
        footer.set_attr("style", "display:none;");
    }
    footer
}

fn main_view(state: &mut AppState, should_display: bool) -> impl View<AppState> + ViewMarker {
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
    let mut toggle_all = el::input(())
        .attr("id", "toggle-all")
        .attr("class", "toggle-all")
        .attr("type", "checkbox");
    if state.are_all_complete() {
        toggle_all.set_attr("checked", "true");
    }
    let mut section = el::section((
        toggle_all.on_click(|state: &mut AppState, _| state.toggle_all_complete()),
        el::label(()).attr("for", "toggle-all"),
        el::ul(todos).attr("class", "todo-list"),
    ))
    .attr("class", "main");
    if !should_display {
        section.set_attr("style", "display:none;");
    }
    section
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    tracing::debug!("render: {state:?}");
    let some_todos = !state.todos.is_empty();
    let main = main_view(state, some_todos);
    let footer = footer_view(state, some_todos);
    let input = el::input(())
        .attr("class", "new-todo")
        .attr("placeholder", "What needs to be done?")
        .attr("value", state.new_todo.clone())
        .attr("autofocus", "true");
    el::div((
        el::header((
            el::h1("TODOs"),
            input
                .on_keydown(|state: &mut AppState, evt| {
                    if evt.key() == "Enter" {
                        state.create_todo();
                    }
                })
                .on_input(|state: &mut AppState, evt| {
                    state.update_new_todo(&evt.target().value());
                    evt.prevent_default();
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
    tracing_wasm::set_as_global_default();
    App::new(AppState::default(), app_logic).run(&get_element_by_id("todoapp"));

    Ok(())
}
