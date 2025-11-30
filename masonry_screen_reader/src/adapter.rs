// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Point, Role, Toggled, TreeUpdate};
use accesskit_consumer::{Node, Tree, TreeChangeHandler};

use crate::filter::filter;

fn describe_role(node: &Node<'_>) -> Option<String> {
    let role_desc = match node.role() {
        Role::Button => "button",
        Role::CheckBox => "checkbox",
        Role::TextInput => "text input",
        Role::MultilineTextInput => "multiline text input",
        Role::Document => "document",
        Role::ProgressIndicator => "progress indicator",
        Role::ScrollBar => "scrollbar",
        Role::ScrollView => "scroll view",
        Role::Splitter => "splitter",
        Role::Image => "image",
        _ => return None,
    };

    Some(role_desc.to_string())
}

fn describe_value(node: &Node<'_>) -> Option<String> {
    node.value()
        .map(|value| {
            if value.is_empty() {
                "blank".to_string()
            } else if node.role() == Role::PasswordInput {
                format!("{} characters", value.len())
            } else {
                value
            }
        })
        .or_else(|| {
            node.numeric_value().map(|numeric_value| {
                let min = node.min_numeric_value();
                let max = node.max_numeric_value();

                match (min, max) {
                    (Some(min), Some(max)) if max > min => {
                        let percentage = ((numeric_value - min) / (max - min)) * 100.0;
                        format!("{:.1}%", percentage)
                    }
                    _ => numeric_value.to_string(),
                }
            })
        })
}

fn describe_state(node: &Node<'_>) -> String {
    let mut states = Vec::new();

    if node.is_disabled() {
        states.push("disabled");
    }

    if node.is_read_only_supported() && node.is_read_only() {
        states.push("readonly");
    }

    if let Some(toggled) = node.toggled() {
        match toggled {
            Toggled::True => states.push("checked"),
            Toggled::False => states.push("unchecked"),
            Toggled::Mixed => states.push("partially checked"),
        }
    }

    states.join(", ")
}

fn describe_node(node: &Node<'_>) -> String {
    let mut parts = Vec::new();

    if !node.label_comes_from_value() {
        if let Some(label) = node.label() {
            parts.push(label);
        }
    } else if let Some(value) = node.value() {
        parts.push(value);
    }

    if let Some(role_desc) = describe_role(node) {
        parts.push(role_desc);
    }

    let state_info = describe_state(node);
    if !state_info.is_empty() {
        parts.push(state_info);
    }

    if let Some(value_info) = describe_value(node) {
        parts.push(value_info);
    }

    if let Some(placeholder) = node.placeholder() {
        parts.push(format!("placeholder: {}", placeholder));
    }

    parts.join(", ")
}

struct ScreenReaderChangeHandler {
    messages: Vec<String>,
}

impl ScreenReaderChangeHandler {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl TreeChangeHandler for ScreenReaderChangeHandler {
    fn node_added(&mut self, _node: &Node<'_>) {}

    fn node_updated(&mut self, old_node: &Node<'_>, new_node: &Node<'_>) {
        if new_node.is_focused() {
            let old_toggled = old_node.toggled();
            let new_toggled = new_node.toggled();

            if old_toggled != new_toggled {
                let description = describe_node(new_node);
                self.messages.push(format!("Updated: {}", description));
            }
        } else if new_node.role() == Role::ProgressIndicator {
            let old_value = old_node.numeric_value();
            let new_value = new_node.numeric_value();

            if old_value != new_value
                && new_value.is_some()
                && let Some(value_desc) = describe_value(new_node)
            {
                self.messages.push(value_desc);
            }
        }
    }

    fn focus_moved(&mut self, _old_node: Option<&Node<'_>>, new_node: Option<&Node<'_>>) {
        if let Some(new_node) = new_node {
            self.messages.push(describe_node(new_node));
        }
    }

    fn node_removed(&mut self, _node: &Node<'_>) {}
}

#[derive(Debug)]
enum State {
    Inactive { is_host_focused: bool },
    Active { tree: Box<Tree> },
}

impl Default for State {
    fn default() -> Self {
        Self::Inactive {
            is_host_focused: false,
        }
    }
}

/// A screen reader simulator that generates human-readable descriptions of accessibility tree changes.
///
/// `ScreenReader` monitors accessibility tree updates and produces text descriptions of what
/// would be announced to screen reader users. It starts in an inactive state and becomes active
/// when the first tree update is received.
#[derive(Debug, Default)]
pub struct ScreenReader {
    state: State,
}

impl ScreenReader {
    /// Creates a new `ScreenReader` in an inactive state.
    ///
    /// The screen reader will become active when the first accessibility tree update is received
    /// via [`update`](Self::update).
    pub fn new() -> Self {
        Self {
            state: State::Inactive {
                is_host_focused: false,
            },
        }
    }

    /// Processes an accessibility tree update and returns descriptions of changes.
    ///
    /// On the first call, this activates the screen reader and initializes it with the provided
    /// tree structure. Subsequent calls process tree changes and generate descriptions.
    ///
    /// # Returns
    ///
    /// A vector of strings describing the changes that would be announced to a screen reader user.
    pub fn update(&mut self, update: TreeUpdate) -> Vec<String> {
        match &mut self.state {
            State::Inactive { is_host_focused } => {
                let tree = Box::new(Tree::new(update, *is_host_focused));

                let messages = if let Some(focused_node) = tree.state().focus() {
                    vec![describe_node(&focused_node)]
                } else {
                    Vec::new()
                };

                self.state = State::Active { tree };
                messages
            }
            State::Active { tree } => {
                let mut change_handler = ScreenReaderChangeHandler::new();
                tree.update_and_process_changes(update, &mut change_handler);
                change_handler.messages
            }
        }
    }

    /// Updates the window focus state and returns any resulting announcements.
    ///
    /// This should be called when the application window gains or loses focus.
    ///
    /// # Arguments
    ///
    /// * `is_focused` - Whether the window has focus
    ///
    /// # Returns
    ///
    /// A vector of strings describing any changes that result from the focus state change.
    pub fn update_window_focus_state(&mut self, is_focused: bool) -> Vec<String> {
        match &mut self.state {
            State::Inactive { is_host_focused } => {
                *is_host_focused = is_focused;
                Vec::new()
            }
            State::Active { tree } => {
                let mut change_handler = ScreenReaderChangeHandler::new();
                tree.update_host_focus_state_and_process_changes(is_focused, &mut change_handler);
                change_handler.messages
            }
        }
    }

    /// Performs a hit test at the given coordinates and returns a description of the element found.
    ///
    /// This simulates what a screen reader would announce when the user touches or hovers over
    /// a specific point in the interface.
    ///
    /// # Arguments
    ///
    /// * `x` - The x coordinate in logical pixels
    /// * `y` - The y coordinate in logical pixels
    ///
    /// # Returns
    ///
    /// An optional string with a description of the element at the given point.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<String> {
        match &self.state {
            State::Inactive { .. } => None,
            State::Active { tree } => {
                let root = tree.state().root();
                let point = Point::new(x, y);
                if let Some(node) = root.node_at_point(point, &filter) {
                    Some(describe_node(&node))
                } else {
                    None
                }
            }
        }
    }
}
