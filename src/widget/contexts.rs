// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Contexts for the widget system.
//!
//! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its contexts.rs.

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use accesskit::TreeUpdate;
use glazier::{
    kurbo::{Point, Rect, Size},
    WindowHandle,
};
use parley::FontContext;

use crate::event::Message;

use super::{PodFlags, WidgetState};

// These contexts loosely follow Druid.
pub struct CxState<'a> {
    window: &'a WindowHandle,
    font_cx: &'a mut FontContext,
    events: &'a mut Vec<Message>,
}

pub struct EventCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) is_handled: bool,
}

pub struct LifeCycleCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct UpdateCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct LayoutCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct AccessCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) update: &'a mut TreeUpdate,
}

pub struct PaintCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a WidgetState,
}

impl<'a> CxState<'a> {
    pub fn new(
        window: &'a WindowHandle,
        font_cx: &'a mut FontContext,
        events: &'a mut Vec<Message>,
    ) -> Self {
        CxState {
            window,
            font_cx,
            events,
        }
    }

    pub(crate) fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}

impl<'a, 'b> EventCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        EventCx {
            cx_state,
            widget_state: root_state,
            is_handled: false,
        }
    }

    pub fn add_event(&mut self, event: Message) {
        self.cx_state.events.push(event);
    }

    pub fn set_active(&mut self, is_active: bool) {
        self.widget_state.flags.set(PodFlags::IS_ACTIVE, is_active);
    }

    pub fn is_hot(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::IS_HOT)
    }

    pub fn set_handled(&mut self, is_handled: bool) {
        self.is_handled = is_handled;
    }

    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// Check whether this widget's id matches the given id.
    pub fn is_accesskit_target(&self, id: accesskit::NodeId) -> bool {
        accesskit::NodeId::from(self.widget_state.id) == id
    }
}

impl<'a, 'b> LifeCycleCx<'a, 'b> {
    pub fn request_paint(&mut self) {
        self.widget_state.flags |= PodFlags::REQUEST_PAINT;
    }
}

impl<'a, 'b> UpdateCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        UpdateCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn request_layout(&mut self) {
        // If the layout changes, the accessibility tree needs to be updated to
        // match. Alternatively, we could be lazy and request accessibility when
        // the layout actually changes.
        self.widget_state.flags |= PodFlags::REQUEST_LAYOUT | PodFlags::REQUEST_ACCESSIBILITY;
    }
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn add_event(&mut self, event: Message) {
        self.cx_state.events.push(event);
    }

    pub fn font_cx(&mut self) -> &mut FontContext {
        self.cx_state.font_cx
    }
}

// This function is unfortunate but works around kurbo versioning
fn to_accesskit_rect(r: Rect) -> accesskit::kurbo::Rect {
    println!("{:?}", r);
    accesskit::kurbo::Rect::new(r.x0, r.y0, r.x1, r.y1)
}

impl<'a, 'b> AccessCx<'a, 'b> {
    /// Add a node to the tree update being built.
    ///
    /// The id of the node pushed is obtained from the context. The
    /// bounds are set based on the layout bounds.
    pub fn push_node(&mut self, mut node: accesskit::Node) {
        node.bounds = Some(to_accesskit_rect(Rect::from_origin_size(
            self.widget_state.window_origin(),
            self.widget_state.size,
        )));
        self.push_node_raw(node);
    }

    /// Add a node to the tree update being built.
    ///
    /// Similar to `push_node` but it is the responsibility of the caller
    /// to set bounds before calling.
    pub fn push_node_raw(&mut self, node: accesskit::Node) {
        let id = self.widget_state.id.into();
        self.update.nodes.push((id, Arc::new(node)));
    }

    /// Report whether accessibility was requested on this widget.
    ///
    /// This method is primarily intended for containers. The `accessibility`
    /// method will be called on a widget when it or any of its descendants
    /// have seen a request. However, in many cases a container need not push
    /// a node for itself.
    pub fn is_requested(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::REQUEST_ACCESSIBILITY)
    }
}

impl<'a, 'b> PaintCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, widget_state: &'a mut WidgetState) -> Self {
        PaintCx {
            cx_state,
            widget_state,
        }
    }

    pub fn is_hot(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::IS_HOT)
    }

    pub fn is_active(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::IS_ACTIVE)
    }

    pub fn size(&self) -> Size {
        self.widget_state.size
    }

    pub fn font_cx(&mut self) -> &mut FontContext {
        self.cx_state.font_cx
    }
}
