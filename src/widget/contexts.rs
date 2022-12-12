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

use std::ops::{Deref, DerefMut};

use glazier::{
    kurbo::{Point, Size},
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
        self.widget_state.flags |= PodFlags::REQUEST_LAYOUT;
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
