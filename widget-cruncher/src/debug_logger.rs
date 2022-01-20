use std::collections::HashMap;
use std::sync::Arc;

use crate::debug_values::{
    LayoutInfo, LayoutTree, LogId, MyWidgetId, Snapshot, StateTree, Timeline, Value,
};
use crate::AsWidgetPod;

#[derive(Debug)]
pub struct DebugLog {
    message: String,
    //children: Vec<LogId>,
}

#[derive(Debug)]
pub struct DebugLogger {
    pub layout_tree: LayoutTree,
    pub widget_states: HashMap<MyWidgetId, StateTree>,
    pub global_state: StateTree,
    pub event_state: StateTree,

    pub logs: HashMap<LogId, DebugLog>,
    pub root_logs: Vec<LogId>,
    pub snapshots: HashMap<LogId, Snapshot>,
    //pub span_stack: Vec<LogId>,
    pub log_id_counter: LogId,
}

impl DebugLogger {
    pub fn new(root_widget: &dyn AsWidgetPod) -> Self {
        let (layout_tree, widget_states) = Self::get_data(root_widget);
        let mut new_self = DebugLogger {
            layout_tree,
            widget_states,
            global_state: Default::default(),
            event_state: Default::default(),
            logs: HashMap::new(),
            root_logs: Vec::new(),
            snapshots: Default::default(),
            log_id_counter: LogId(0),
        };
        new_self.push_log("initial value");
        new_self
    }

    pub fn write_to_file(&self, path: &str) {
        use std::fs::File;
        use std::io::{BufReader, BufWriter, Write};

        fn add_logs(tree: &mut StateTree, logs: &HashMap<LogId, DebugLog>, log_ids: &[LogId]) {
            let mut children = Vec::new();
            for log in log_ids {
                let mut child = StateTree {
                    name: logs[log].message.clone(),
                    value: (*log).into(),
                    children: Default::default(),
                };
                //add_logs(&mut child, logs, &logs[log].children);
                children.push(child);
            }
            tree.children = children.into();
        }

        let mut log_tree = StateTree {
            name: "Logs".to_string(),
            value: Value::Empty,
            children: Default::default(),
        };
        add_logs(&mut log_tree, &self.logs, &self.root_logs);

        let timeline = Timeline {
            logs: log_tree,
            snapshots: self.snapshots.clone(),
            // TODO - for now we start with LogId(1)
            selected_log: LogId(1),
        };

        let file = File::create(path).unwrap();
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &timeline).unwrap();
        writer.flush().unwrap();
    }

    pub fn push_log(&mut self, message: &str) {
        self.push_snapshot();
        self.logs.insert(
            self.log_id_counter,
            DebugLog {
                message: message.to_string(),
            },
        );
        self.root_logs.push(self.log_id_counter);
    }

    fn push_snapshot(&mut self) {
        self.log_id_counter.0 += 1;
        self.snapshots.insert(
            self.log_id_counter,
            Snapshot {
                layout_tree: self.layout_tree.clone(),
                widget_states: self.widget_states.clone(),
                global_state: self.global_state.clone(),
                event_state: self.event_state.clone(),
                selected_widget: 0,
            },
        );
    }

    pub fn get_widget_state(widget: &dyn AsWidgetPod) -> StateTree {
        let mut state = StateTree::default();
        let w_state = widget.state();

        // TODO
        #[cfg(debug_assertions)]
        {
            state.name = w_state.widget_name.to_string();
        }

        state.children = vec![
            StateTree::new(
                "is_expecting_set_origin_call",
                w_state.is_expecting_set_origin_call,
            ),
            StateTree::new("is_new", w_state.is_new),
            StateTree::new(
                "children_disabled_changed",
                w_state.children_disabled_changed,
            ),
            StateTree::new("ancestor_disabled", w_state.ancestor_disabled),
            StateTree::new("is_explicitly_disabled", w_state.is_explicitly_disabled),
            StateTree::new("is_hot", w_state.is_hot),
            StateTree::new("needs_layout", w_state.needs_layout),
            StateTree::new("needs_window_origin", w_state.needs_window_origin),
            StateTree::new("is_active", w_state.is_active),
            StateTree::new("has_active", w_state.has_active),
            StateTree::new("has_focus", w_state.has_focus),
            StateTree::new("request_anim", w_state.request_anim),
            StateTree::new("children_changed", w_state.children_changed),
            StateTree::new(
                "is_explicitly_disabled_new",
                w_state.is_explicitly_disabled_new,
            ),
            StateTree::new("update_focus_chain", w_state.update_focus_chain),
        ]
        .into();
        state
    }

    pub fn get_data(root_widget: &dyn AsWidgetPod) -> (LayoutTree, HashMap<MyWidgetId, StateTree>) {
        fn add_to_tree(
            widgets_map: &mut HashMap<MyWidgetId, LayoutInfo>,
            widget_states: &mut HashMap<MyWidgetId, StateTree>,
            widget: &dyn AsWidgetPod,
        ) {
            let mut layout_info = LayoutInfo {
                layout_rect: widget.state().layout_rect(),
                typename: widget.widget().short_type_name().into(),
                children: Default::default(),
            };

            for child in widget.children() {
                let child_id = child.state().id.to_raw() as u32;
                layout_info.children.insert(child_id);
                add_to_tree(widgets_map, widget_states, child);
            }

            let id = widget.state().id.to_raw() as u32;
            widgets_map.insert(id, layout_info);
            widget_states.insert(id, DebugLogger::get_widget_state(widget));
        }

        let mut widgets_map = HashMap::new();
        let mut widget_states = HashMap::new();
        add_to_tree(&mut widgets_map, &mut widget_states, root_widget);

        (
            LayoutTree {
                root: root_widget.state().id.to_raw() as u32,
                widgets: Arc::new(widgets_map),
            },
            widget_states,
        )
    }
}
