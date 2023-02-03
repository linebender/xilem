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

use std::sync::Arc;

use accesskit::TreeUpdate;
use glazier::{
    kurbo::{Rect, Size},
    WindowHandle,
};
use parley::FontContext;

use crate::event::Message;

use super::{PodFlags, WidgetState};

// These contexts loosely follow Druid.
pub struct CxState<'a> {
    window: &'a WindowHandle,
    font_cx: &'a mut FontContext,
    messages: &'a mut Vec<Message>,
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
    pub(crate) node_classes: &'a mut accesskit::NodeClassSet,
}

pub struct PaintCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a WidgetState,
}

/// A macro for implementing methods on multiple contexts.
///
/// There are a lot of methods defined on multiple contexts; this lets us only
/// have to write them out once.
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        impl $ty { $($method)+ }
    };
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

impl<'a> CxState<'a> {
    pub fn new(
        window: &'a WindowHandle,
        font_cx: &'a mut FontContext,
        messages: &'a mut Vec<Message>,
    ) -> Self {
        CxState {
            window,
            font_cx,
            messages,
        }
    }

    pub(crate) fn has_messages(&self) -> bool {
        !self.messages.is_empty()
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

    pub fn set_active(&mut self, is_active: bool) {
        self.widget_state.flags.set(PodFlags::IS_ACTIVE, is_active);
    }

    pub fn request_update(&mut self) {
        self.widget_state.flags.insert(PodFlags::REQUEST_UPDATE);
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
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LifeCycleCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b> UpdateCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        UpdateCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn font_cx(&mut self) -> &mut FontContext {
        self.cx_state.font_cx
    }
}

// This function is unfortunate but works around kurbo versioning
fn to_accesskit_rect(r: Rect) -> accesskit::Rect {
    println!("{:?}", r);
    accesskit::Rect::new(r.x0, r.y0, r.x1, r.y1)
}

impl<'a, 'b> AccessCx<'a, 'b> {
    /// Add a node to the tree update being built.
    ///
    /// The id of the node pushed is obtained from the context. The
    /// bounds are set based on the layout bounds.
    pub fn push_node(&mut self, mut builder: accesskit::NodeBuilder) {
        builder.set_bounds(to_accesskit_rect(Rect::from_origin_size(
            self.widget_state.window_origin(),
            self.widget_state.size,
        )));
        let node = builder.build(&mut self.node_classes);
        self.push_node_raw(node);
    }

    /// Add a node to the tree update being built.
    ///
    /// Similar to `push_node` but it is the responsibility of the caller
    /// to set bounds before calling.
    pub fn push_node_raw(&mut self, node: accesskit::Node) {
        let id = self.widget_state.id.into();
        self.update.nodes.push((id, node));
    }

    /// Report whether accessibility was requested on this widget.
    ///
    /// This method is primarily intended for containers. The `accessibility`
    /// method will be called on a widget when it or any of its descendants
    /// have seen a request. However, in many cases a container need not push
    /// a node for itself.
    pub fn is_requested(&self) -> bool {
        self.widget_state
            .flags
            .contains(PodFlags::REQUEST_ACCESSIBILITY)
    }
}

impl<'a, 'b> PaintCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, widget_state: &'a mut WidgetState) -> Self {
        PaintCx {
            cx_state,
            widget_state,
        }
    }

    pub fn font_cx(&mut self) -> &mut FontContext {
        self.cx_state.font_cx
    }
}

// Methods on all contexts.
//
// These Methods return information about the widget
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    AccessCx<'_, '_>,
    PaintCx<'_, '_>,
    {
        pub fn is_hot(&self) -> bool {
            self.widget_state.flags.contains(PodFlags::IS_HOT)
        }

        pub fn is_active(&self) -> bool {
            self.widget_state.flags.contains(PodFlags::IS_ACTIVE)
        }
    }
);

// Methods on EventCx, UpdateCx, and LifeCycleCx
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    {
        pub fn request_layout(&mut self) {
            // If the layout changes, the accessibility tree needs to be updated to
            // match. Alternatively, we could be lazy and request accessibility when
            // the layout actually changes.
            self.widget_state.flags |= PodFlags::REQUEST_LAYOUT | PodFlags::REQUEST_ACCESSIBILITY;
        }

        pub fn add_message(&mut self, message: Message) {
            self.cx_state.messages.push(message);
        }
    }
);

// Methods on EventCx, UpdateCx, LifeCycleCx and LayoutCx
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    {
        pub fn request_paint(&mut self) {
            self.widget_state.flags |= PodFlags::REQUEST_PAINT;
        }

        pub fn view_context_changed(&mut self) {
            self.widget_state.flags |= PodFlags::VIEW_CONTEXT_CHANGED;
        }
    }
);

// Methods on all contexts besides LayoutCx.
//
// These Methods return information about the widget
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    AccessCx<'_, '_>,
    PaintCx<'_, '_>,
    {
        pub fn size(&self) -> Size {
            self.widget_state.size
        }
    }
);