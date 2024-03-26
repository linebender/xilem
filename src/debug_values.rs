// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(clippy::all)]
#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;

use crate::Rect;
use serde::{Deserialize, Serialize};

pub type MyWidgetId = u32;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct LogId(pub i32);

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Value {
    Empty,
    String(String),
    Bool(bool),
    #[serde(with = "serde_rect")]
    Rect(Rect),
    Id(MyWidgetId),
    LogId(LogId),
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct StateTree {
    pub name: String,
    pub value: Value,
    pub folded_by_default: bool,
    #[serde(with = "serde_arc")]
    pub children: Arc<Vec<StateTree>>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct LayoutInfo {
    #[serde(with = "serde_rect")]
    pub layout_rect: Rect,
    #[serde(default)]
    pub typename: String,
    pub children: HashSet<MyWidgetId>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct LayoutTree {
    pub root: Option<MyWidgetId>,
    #[serde(with = "serde_arc")]
    pub widgets: Arc<HashMap<MyWidgetId, LayoutInfo>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub layout_tree: LayoutTree,
    pub widget_states: HashMap<MyWidgetId, StateTree>,
    pub global_state: StateTree,
    pub event_state: StateTree,
    #[serde(default)]
    pub selected_widget: MyWidgetId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timeline {
    pub logs: StateTree,
    pub snapshots: HashMap<LogId, Snapshot>,
    #[serde(default)]
    pub selected_log: LogId,
}

// ---

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Empty => write!(f, ""),
            Value::String(string) => write!(f, "{}", string),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Rect(rect) => write!(f, "{:?}", rect),
            Value::Id(id) => write!(f, "{}", id),
            Value::LogId(_) => write!(f, "<snapshot>"),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Empty
    }
}

impl From<String> for Value {
    fn from(value: String) -> Value {
        Value::String(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Value {
        Value::Bool(value)
    }
}

impl From<Rect> for Value {
    fn from(value: Rect) -> Value {
        Value::Rect(value)
    }
}

impl From<MyWidgetId> for Value {
    fn from(value: MyWidgetId) -> Value {
        Value::Id(value)
    }
}

impl From<LogId> for Value {
    fn from(value: LogId) -> Value {
        Value::LogId(value)
    }
}

impl StateTree {
    pub fn new(name: impl Into<String>, value: impl Into<Value>) -> Self {
        StateTree {
            name: name.into(),
            value: value.into(),
            folded_by_default: false,
            children: vec![].into(),
        }
    }
}

impl Snapshot {
    pub fn get_selected_state(&self) -> &StateTree {
        self.widget_states
            .get(&self.selected_widget)
            .unwrap_or(&self.global_state)
    }

    pub fn get_selected_state_mut(&mut self) -> &mut StateTree {
        self.widget_states
            .get_mut(&mut self.selected_widget)
            .unwrap_or(&mut self.global_state)
    }
}

impl Timeline {
    pub fn get_selected_snapshot(&self) -> &Snapshot {
        self.snapshots.get(&self.selected_log).unwrap()
    }

    pub fn get_selected_snapshot_mut(&mut self) -> &mut Snapshot {
        self.snapshots.get_mut(&mut self.selected_log).unwrap()
    }
}

mod serde_arc {
    use std::sync::Arc;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T: Serialize, S: Serializer>(
        value: &Arc<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Arc<T>, D::Error> {
        let value = Deserialize::deserialize(deserializer)?;
        Ok(Arc::new(value))
    }
}

mod serde_rect {
    use crate::Rect;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(value: &Rect, serializer: S) -> Result<S::Ok, S::Error> {
        let value = (value.x0, value.y0, value.x1, value.y1);
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rect, D::Error> {
        let (x0, y0, x1, y1) = Deserialize::deserialize(deserializer)?;
        Ok(Rect::new(x0, y0, x1, y1))
    }
}
