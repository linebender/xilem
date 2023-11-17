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
use vello::kurbo::{Affine, Point, Size};
use vello::peniko::Mix;
use vello::SceneBuilder;

use super::{BoxConstraints, Widget};

use super::{contexts::LifeCycleCx, Event, EventCx, LayoutCx, LifeCycle, PaintCx, Pod, UpdateCx};

pub struct ScrollView {
    child: Pod,
    offset: f64,
}

impl ScrollView {
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollView {
            child: Pod::new(child),
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
        // Pass event through to child
        self.child.event(cx, event);

        // Handle scroll wheel events
        if !cx.is_handled() {
            if let Event::MouseWheel(mouse) = event {
                let max_offset = (self.child.get_size().height - cx.size().height).max(0.0);
                let new_offset = (self.offset + mouse.wheel_delta.y).max(0.0).min(max_offset);
                if new_offset != self.offset {
                    self.offset = new_offset;
                    let new_origin = Point {
                        x: 0.0,
                        y: -self.offset,
                    };
                    self.child.set_origin(cx.widget_state, new_origin);
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
            Size {
                width: 0.0,
                height: 0.0,
            },
            Size {
                width: bc.max().width,
                height: f64::INFINITY,
            },
        );
        let child_size = self.child.layout(cx, &cbc);
        let size = Size {
            width: child_size.width.min(bc.max().width),
            height: child_size.height.min(bc.max().height),
        };

        // Ensure that scroll offset is within bounds
        let max_offset = (child_size.height - size.height).max(0.0);
        if max_offset < self.offset {
            self.offset = max_offset;
            let new_origin = Point {
                x: 0.0,
                y: -self.offset,
            };
            self.child.set_origin(cx.widget_state, new_origin);
        }

        size
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        builder.push_layer(Mix::Normal, 1.0, Affine::IDENTITY, &cx.size().to_rect());
        self.child.paint(cx, builder);
        builder.pop_layer();
    }

    fn accessibility(&mut self, cx: &mut super::AccessCx) {
        self.child.accessibility(cx);

        if cx.is_requested() {
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::ScrollView);
            builder.set_children(vec![self.child.id().into()]);
            cx.push_node(builder);
        }
    }
}
