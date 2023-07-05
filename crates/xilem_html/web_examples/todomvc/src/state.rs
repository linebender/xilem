use std::sync::atomic::{AtomicU64, Ordering};

fn next_id() -> u64 {
    static ID_GEN: AtomicU64 = AtomicU64::new(1);
    ID_GEN.fetch_add(1, Ordering::Relaxed)
}

#[derive(Default, Debug)]
pub struct AppState {
    pub new_todo: String,
    pub todos: Vec<Todo>,
    pub filter: Filter,
    pub editing_id: Option<u64>,
    pub focus_new_todo: bool,
}

impl AppState {
    pub fn create_todo(&mut self) {
        if self.new_todo.is_empty() {
            return;
        }
        let title = self.new_todo.trim().to_string();
        self.new_todo.clear();
        self.todos.push(Todo::new(title));
        self.focus_new_todo = true;
    }

    pub fn visible_todos(&mut self) -> impl Iterator<Item = (usize, &mut Todo)> {
        self.todos
            .iter_mut()
            .enumerate()
            .filter(|(_, todo)| match self.filter {
                Filter::All => true,
                Filter::Active => !todo.completed,
                Filter::Completed => todo.completed,
            })
    }

    pub fn update_new_todo(&mut self, new_text: &str) {
        self.new_todo.clear();
        self.new_todo.push_str(new_text);
    }

    pub fn start_editing(&mut self, id: u64) {
        if let Some(ref mut todo) = self.todos.iter_mut().filter(|todo| todo.id == id).next() {
            todo.title_editing.clear();
            todo.title_editing.push_str(&todo.title);
            self.editing_id = Some(id)
        }
    }
}

#[derive(Debug)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub title_editing: String,
    pub completed: bool,
}

impl Todo {
    pub fn new(title: String) -> Self {
        let title_editing = title.clone();
        Self {
            id: next_id(),
            title,
            title_editing,
            completed: false,
        }
    }

    pub fn save_editing(&mut self) {
        self.title.clear();
        self.title.push_str(&self.title_editing);
    }
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub enum Filter {
    #[default]
    All,
    Active,
    Completed,
}
