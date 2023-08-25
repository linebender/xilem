mod state;

use state::{AppState, Filter, Todo};

use wasm_bindgen::JsCast;
use xilem_html::{
    dom::{
        elements::{self as el},
        event::EventListenerOptions,
        interfaces::*,
    },
    // events::on_click,
    get_element_by_id,
    Action,
    Adapt,
    App,
    MessageResult,
    View,
};

// All of these actions arise from within a `Todo`, but we need access to the full state to reduce
// them.
enum TodoAction {
    SetEditing(u64),
    CommitEdit,
    CancelEditing,
    Destroy(u64),
}

impl Action for TodoAction {}

fn todo_item(todo: &mut Todo, editing: bool) -> impl Element<Todo, TodoAction> {
    let mut class = String::new();
    if todo.completed {
        class.push_str(" completed");
    }
    if editing {
        class.push_str(" editing");
    }

    let checkbox = el::input(())
        .attr("class", "toggle")
        .attr("type", "checkbox")
        .attr("checked", todo.completed)
        .on_click(|state: &mut Todo, _| state.completed = !state.completed);

    el::li((
        el::div((
            checkbox,
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
                    Some(TodoAction::CommitEdit)
                } else if key == "Escape" {
                    Some(TodoAction::CancelEditing)
                } else {
                    None
                }
            })
            .on_input_with_options(
                |state: &mut Todo, evt| {
                    // TODO is there a less boilerplate but safe way to get to the value of the element?
                    let Some(target) = evt.target() else {
                        return;
                    };
                    let Some(element) = target.dyn_ref::<web_sys::HtmlInputElement>() else {
                        return;
                    };
                    evt.prevent_default();
                    state.title_editing = element.value();
                },
                EventListenerOptions::enable_prevent_default(),
            )
            .on_blur(|_, _| TodoAction::CancelEditing),
    ))
    .attr("class", class)
}

fn footer_view(state: &mut AppState, should_display: bool) -> impl Element<AppState> {
    let item_str = if state.todos.len() == 1 {
        "item"
    } else {
        "items"
    };

    let clear_button = (state.todos.iter().filter(|todo| todo.completed).count() > 0).then(|| {
        Element::on_click(
            el::button("Clear completed").attr("class", "clear-completed"),
            |state: &mut AppState, _| {
                state.todos.retain(|todo| !todo.completed);
            },
        )
    });

    let filter_class = |filter| (state.filter == filter).then_some("selected");

    el::footer((
        el::span((
            el::strong(state.todos.len().to_string()),
            format!(" {} left", item_str),
        ))
        .attr("class", "todo-count"),
        el::ul((
            el::li(Element::on_click(
                el::a("All")
                    .attr("href", "#/")
                    .attr("class", filter_class(Filter::All)),
                |state: &mut AppState, _| {
                    state.filter = Filter::All;
                },
            )),
            " ",
            el::li(Element::on_click(
                el::a("Active")
                    .attr("href", "#/active")
                    .attr("class", filter_class(Filter::Active)),
                |state: &mut AppState, _| {
                    state.filter = Filter::Active;
                },
            )),
            " ",
            el::li(Element::on_click(
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
    .attr("class", "footer")
    .attr("style", (!should_display).then_some("display:none;"))
}

fn main_view(state: &mut AppState, should_display: bool) -> impl Element<AppState> {
    let editing_id = state.editing_id;
    let todos: Vec<_> = state
        .visible_todos()
        .map(|(idx, todo)| {
            Adapt::new(
                move |data: &mut AppState, thunk| {
                    if let MessageResult::Action(action) = thunk.call(&mut data.todos[idx]) {
                        match action {
                            TodoAction::SetEditing(id) => data.start_editing(id),
                            TodoAction::CommitEdit => {
                                data.save();
                                data.editing_id = None;
                            }
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
    let toggle_all = el::input(())
        .attr("id", "toggle-all")
        .attr("class", "toggle-all")
        .attr("type", "checkbox")
        .attr("checked", state.are_all_complete());

    el::section((
        toggle_all.on_click(|state: &mut AppState, _| state.toggle_all_complete()),
        el::label(()).attr("for", "toggle-all"),
        el::ul(todos).attr("class", "todo-list"),
    ))
    .attr("class", "main")
    .attr("style", (!should_display).then_some("display:none;"))
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
        .attr("autofocus", true);
    el::div((
        el::header((
            el::h1("TODOs"),
            input
                .on_keydown(|state: &mut AppState, evt| {
                    if evt.key() == "Enter" {
                        state.create_todo();
                    }
                })
                .on_input_with_options(
                    |state: &mut AppState, evt| {
                        // TODO is there a less boilerplate but safe way to get to the value of the element?
                        let Some(target) = evt.target() else {
                            return;
                        };
                        let Some(element) = target.dyn_ref::<web_sys::HtmlInputElement>() else {
                            return;
                        };
                        state.update_new_todo(&element.value());
                        evt.prevent_default();
                    },
                    EventListenerOptions::enable_prevent_default(),
                ),
        ))
        .attr("class", "header"),
        main,
        footer,
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    App::new(AppState::load(), app_logic).run(&get_element_by_id("todoapp"));
}
