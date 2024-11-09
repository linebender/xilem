// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(clippy::partial_pub_fields, reason = "Deferred: Noisy")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;

const KEY: &str = "todomvc_persist";

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct AppState {
    #[serde(skip)]
    pub new_todo: String,
    pub todos: Vec<Todo>,
    #[serde(skip)]
    pub filter: Filter,
    #[serde(skip)]
    pub editing_id: Option<u64>,
    #[serde(skip)]
    pub focus_new_todo: bool,
    next_id: u64,
}

impl AppState {
    pub(crate) fn create_todo(&mut self) {
        if self.new_todo.is_empty() {
            return;
        }
        let title = self.new_todo.trim().to_string();
        self.new_todo.clear();
        let id: u64 = self.next_id();
        self.todos.push(Todo::new(title, id));
        self.focus_new_todo = true;
        self.save();
    }

    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Are all the todos complete?
    pub(crate) fn are_all_complete(&self) -> bool {
        self.todos.iter().all(|todo| todo.completed)
    }

    /// If all TODOs are complete, then mark them all not complete,
    /// else mark them all complete.
    pub(crate) fn toggle_all_complete(&mut self) {
        if self.are_all_complete() {
            for todo in self.todos.iter_mut() {
                todo.completed = false;
            }
        } else {
            for todo in self.todos.iter_mut() {
                todo.completed = true;
            }
        }
        self.save();
    }

    pub(crate) fn visible_todos(&mut self) -> impl Iterator<Item = (usize, &mut Todo)> {
        self.todos
            .iter_mut()
            .enumerate()
            .filter(|(_, todo)| match self.filter {
                Filter::All => true,
                Filter::Active => !todo.completed,
                Filter::Completed => todo.completed,
            })
    }

    pub(crate) fn update_new_todo(&mut self, new_text: &str) {
        self.new_todo.clear();
        self.new_todo.push_str(new_text);
    }

    pub(crate) fn start_editing(&mut self, id: u64) {
        if let Some(ref mut todo) = self.todos.iter_mut().find(|todo| todo.id == id) {
            todo.title_editing.clear();
            todo.title_editing.push_str(&todo.title);
            self.editing_id = Some(id);
        }
    }

    /// Load the current state from local storage, or use the default.
    pub(crate) fn load() -> Self {
        let Some(raw) = storage().get_item(KEY).unwrap_throw() else {
            return Default::default();
        };
        match serde_json::from_str(&raw) {
            Ok(todos) => todos,
            Err(e) => {
                tracing::error!("couldn't load existing todos: {e}");
                Default::default()
            }
        }
    }

    /// Save the current state to local storage
    pub(crate) fn save(&self) {
        let raw = serde_json::to_string(self).unwrap_throw();
        storage().set_item(KEY, &raw).unwrap_throw();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Todo {
    pub id: u64,
    pub title: String,
    #[serde(skip)]
    pub title_editing: String,
    pub completed: bool,
}

impl Todo {
    pub(crate) fn new(title: String, id: u64) -> Self {
        let title_editing = title.clone();
        Self {
            id,
            title,
            title_editing,
            completed: false,
        }
    }

    pub(crate) fn save_editing(&mut self) {
        self.title.clear();
        self.title.push_str(&self.title_editing);
    }
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub(crate) enum Filter {
    #[default]
    All,
    Active,
    Completed,
}

fn storage() -> web_sys::Storage {
    web_sys::window()
        .unwrap_throw()
        .local_storage()
        .unwrap_throw()
        .unwrap_throw()
}
