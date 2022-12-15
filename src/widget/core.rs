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

//! Core types and mechanisms for the widget hierarchy.
//!
//! //! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its core.rs.

use bitflags::bitflags;
use glazier::kurbo::{Point, Rect, Size};
use vello::{SceneBuilder, SceneFragment};

use crate::Widget;

use super::{
    contexts::LifeCycleCx, AnyWidget, BoxConstraints, CxState, Event, EventCx, LayoutCx, LifeCycle,
    PaintCx, UpdateCx,
};

bitflags! {
    #[derive(Default)]
    pub(crate) struct PodFlags: u32 {
        const REQUEST_UPDATE = 1;
        const REQUEST_LAYOUT = 2;
        const REQUEST_PAINT = 4;

        const IS_HOT = 8;
        const IS_ACTIVE = 16;
        const HAS_ACTIVE = 32;

        const UPWARD_FLAGS = Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits | Self::HAS_ACTIVE.bits;
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits | Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
    }
}

bitflags! {
    #[derive(Default)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const PAINT = 4;
    }
}

/// A pod that contains a widget (in a container).
pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
    fragment: SceneFragment,
}

#[derive(Default, Debug)]
pub(crate) struct WidgetState {
    pub(crate) flags: PodFlags,
    pub(crate) origin: Point,
    /// The minimum intrinsic size of the widget.
    pub(crate) min_size: Size,
    /// The maximum intrinsic size of the widget.
    pub(crate) max_size: Size,
    /// The size proposed by the widget's container.
    pub(crate) proposed_size: Size,
    /// The size of the widget.
    pub(crate) size: Size,
}

impl WidgetState {
    fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags & PodFlags::UPWARD_FLAGS;
    }

    fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }
}

impl Pod {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::new_from_box(Box::new(widget))
    }

    pub fn new_from_box(widget: Box<dyn AnyWidget>) -> Self {
        Pod {
            state: WidgetState {
                flags: PodFlags::INIT_FLAGS,
                ..Default::default()
            },
            fragment: SceneFragment::default(),
            widget,
        }
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    pub fn mark(&mut self, flags: ChangeFlags) {
        self.state
            .request(PodFlags::from_bits(flags.bits().into()).unwrap());
    }

    pub fn request_update(&mut self) {
        self.state.request(PodFlags::REQUEST_UPDATE);
    }

    /// Propagate a platform event. As in Druid, a great deal of the event
    /// dispatching logic is in this function.
    pub fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if cx.is_handled {
            return;
        }
        let rect = Rect::from_origin_size(self.state.origin, self.state.size);
        let mut modified_event = None;
        let had_active = self.state.flags.contains(PodFlags::HAS_ACTIVE);
        let recurse = match event {
            Event::MouseDown(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    rect,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseDown(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseUp(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    rect,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseUp(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseMove(mouse_event) => {
                let hot_changed = Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    rect,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) || hot_changed {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseMove(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseWheel(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    rect,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseWheel(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseLeft() => {
                let hot_changed =
                    Pod::set_hot_state(&mut self.widget, &mut self.state, cx.cx_state, rect, None);
                if had_active || hot_changed {
                    true
                } else {
                    false
                }
            }
        };
        if recurse {
            let mut inner_cx = EventCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
                is_handled: false,
            };
            self.widget
                .event(&mut inner_cx, modified_event.as_ref().unwrap_or(event));
            cx.is_handled |= inner_cx.is_handled;
            self.state.flags.set(
                PodFlags::HAS_ACTIVE,
                self.state.flags.contains(PodFlags::IS_ACTIVE),
            );
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    pub fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        let recurse = match event {
            LifeCycle::HotChanged(_) => false,
        };
        let mut child_cx = LifeCycleCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        if recurse {
            self.widget.lifecycle(&mut child_cx, event);
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Propagate an update cycle.
    pub fn update(&mut self, cx: &mut UpdateCx) {
        if self.state.flags.contains(PodFlags::REQUEST_UPDATE) {
            let mut child_cx = UpdateCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            self.widget.update(&mut child_cx);
            self.state.flags.remove(PodFlags::REQUEST_UPDATE);
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    pub fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        if self.state.flags.contains(PodFlags::REQUEST_LAYOUT) {
            let mut child_cx = LayoutCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            let new_size = self.widget.layout(&mut child_cx, bc);
            println!("layout size = {:?}", new_size);
            self.state.size = new_size;
            self.state.flags.remove(PodFlags::REQUEST_LAYOUT);
        }
        self.state.size
    }

    pub fn paint_raw(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        let mut inner_cx = PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        self.widget.paint(&mut inner_cx, builder);
    }

    pub fn paint(&mut self, cx: &mut PaintCx) {
        let mut inner_cx = PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        let mut builder = SceneBuilder::for_fragment(&mut self.fragment);
        self.widget.paint(&mut inner_cx, &mut builder);
    }

    pub fn height_flexibility(&self) -> f64 {
        self.state.max_size.height - self.state.min_size.height
    }

    // Return true if hot state has changed
    fn set_hot_state(
        widget: &mut dyn AnyWidget,
        widget_state: &mut WidgetState,
        cx_state: &mut CxState,
        rect: Rect,
        mouse_pos: Option<Point>,
    ) -> bool {
        let had_hot = widget_state.flags.contains(PodFlags::IS_HOT);
        let is_hot = match mouse_pos {
            Some(pos) => rect.contains(pos),
            None => false,
        };
        widget_state.flags.set(PodFlags::IS_HOT, is_hot);
        if had_hot != is_hot {
            let hot_changed_event = LifeCycle::HotChanged(is_hot);
            let mut child_cx = LifeCycleCx {
                cx_state,
                widget_state,
            };
            widget.lifecycle(&mut child_cx, &hot_changed_event);
            return true;
        }
        false
    }

    /// Get the rendered scene fragment for the widget.
    ///
    /// This is only valid after a `paint` call, but the fragment can be retained
    /// (skipping further paint calls) if the appearance does not change.
    pub fn fragment(&self) -> &SceneFragment {
        &self.fragment
    }
}
