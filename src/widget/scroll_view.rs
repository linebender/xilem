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

//! A simple scroll view.
//!
//! There's a lot more functionality in the Druid version, including
//! control over scrolling axes, ability to scroll to content, etc.

use crate::Axis;
use crate::id::Id;
use glazier::kurbo::Vec2;
use vello::kurbo::{Affine, Size};
use vello::peniko::Mix;
use vello::Scene;

use super::{BoxConstraints, Widget};

use super::{contexts::LifeCycleCx, Event, EventCx, LayoutCx, LifeCycle, PaintCx, Pod, UpdateCx};

pub struct ScrollView {
    child: Pod,
    offset: f64,
}

impl ScrollView {
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollView {
            child: Pod::new(child, Id::next()),
            offset: 0.0,
        }
    }

    pub fn child_mut(&mut self) -> &mut Pod {
        &mut self.child
    }
}

// TODO: scroll bars
impl Widget for ScrollView {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        // Pass event through to child, adjusting the coordinates of mouse events
        // by the scroll offset first.
        // TODO: scroll wheel + click-drag on scroll bars
        let offset = Vec2::new(0.0, self.offset);
        let child_event = match event {
            Event::MouseDown(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseDown(mouse_event)
            }
            Event::MouseUp(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseUp(mouse_event)
            }
            Event::MouseMove(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseMove(mouse_event)
            }
            Event::MouseWheel(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseWheel(mouse_event)
            }
            _ => event.clone(),
        };

        self.child.event(cx, &child_event);

        // Handle scroll wheel events
        if !cx.is_handled() {
            if let Event::MouseWheel(mouse) = event {
                let max_offset = (self.child.size().height - cx.size().height).max(0.0);
                let new_offset = (self.offset + mouse.wheel_delta.y).max(0.0).min(max_offset);
                if new_offset != self.offset {
                    self.offset = new_offset;
                    cx.set_handled(true);
                    cx.request_paint();
                }
            }
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.child.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.child.update(cx);
    }

    fn compute_max_intrinsic(&mut self, axis: Axis, cx: &mut LayoutCx, bc: &BoxConstraints) -> f64 {
        match axis {
            Axis::Horizontal => {
                if bc.min().width.is_sign_negative() {
                    0.0
                } else {
                    let length =
                        self.child
                            .compute_max_intrinsic(axis, cx, &bc.unbound_max_height());
                    length.min(bc.max().width).max(bc.min().width)
                }
            }
            Axis::Vertical => {
                if bc.min().height.is_sign_negative() {
                    0.0
                } else {
                    let length =
                        self.child
                            .compute_max_intrinsic(axis, cx, &bc.unbound_max_height());
                    length.min(bc.max().height).max(bc.min().height)
                }
            }
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        cx.request_paint();

        let cbc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            Size::new(bc.max().width, f64::INFINITY),
        );
        let child_size = self.child.layout(cx, &cbc);
        let size = Size::new(
            child_size.width.min(bc.max().width),
            child_size.height.min(bc.max().height),
        );

        // Ensure that scroll offset is within bounds
        let max_offset = (child_size.height - size.height).max(0.0);
        if max_offset < self.offset {
            self.offset = max_offset;
        }

        size
    }

    fn paint(&mut self, cx: &mut PaintCx, scene: &mut Scene) {
        scene.push_layer(Mix::Normal, 1.0, Affine::IDENTITY, &cx.size().to_rect());
        let fragment = self.child.paint_custom(cx);
        scene.append(fragment, Some(Affine::translate((0.0, -self.offset))));
        scene.pop_layer();
    }
}
