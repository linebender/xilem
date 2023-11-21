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

use glazier::kurbo::Circle;
use vello::{
    kurbo::{Point, Size},
    peniko::Color,
    SceneBuilder,
};

use crate::{IdPath, Message};

use super::{
    contexts::LifeCycleCx,
    piet_scene_helpers::{fill_color, stroke},
    AccessCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle, PaintCx, UpdateCx,
    Widget,
};

pub struct Switch {
    id_path: IdPath,
    is_on: bool,
    is_moved: bool,
    knob_position: Point,
}

impl Switch {
    pub fn new(id_path: &IdPath, is_on: bool) -> Switch {
        Switch {
            id_path: id_path.clone(),
            is_on,
            is_moved: false,
            knob_position: if is_on {
                Point::new(ON_POS, KNOB_DIAMETER / 2. + SWITCH_PADDING)
            } else {
                Point::new(OFF_POS, KNOB_DIAMETER / 2. + SWITCH_PADDING)
            },
        }
    }

    pub fn set_is_on(&mut self, is_on: bool) -> ChangeFlags {
        self.is_on = is_on;
        ChangeFlags::PAINT
    }
}

// See druid's button for info.
const KNOB_DIAMETER: f64 = 20.0;
const SWITCH_PADDING: f64 = 3.0;
const SWITCH_WIDTH: f64 = 2.0 * KNOB_DIAMETER + 2.0 * SWITCH_PADDING;
const SWITCH_HEIGHT: f64 = KNOB_DIAMETER + 2.0 * SWITCH_PADDING;
const ON_POS: f64 = SWITCH_WIDTH - KNOB_DIAMETER / 2.0 - SWITCH_PADDING;
const OFF_POS: f64 = KNOB_DIAMETER / 2.0 + SWITCH_PADDING;

impl Widget for Switch {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        match event {
            Event::MouseDown(_) => {
                cx.set_active(true);
                cx.request_paint();
            }
            Event::MouseUp(_) => {
                if self.is_moved {
                    if self.is_on != (self.knob_position.x > SWITCH_WIDTH / 2.0) {
                        cx.add_message(Message::new(self.id_path.clone(), ()))
                    }
                } else if cx.is_active() {
                    cx.add_message(Message::new(self.id_path.clone(), ()));
                }
                // Reset Flags
                cx.set_active(false);
                self.is_moved = false;

                // Request repaint
                cx.request_paint();
            }
            Event::MouseMove(mouse) => {
                if cx.is_active() {
                    self.knob_position.x = mouse.pos.x.clamp(OFF_POS, ON_POS);
                    self.is_moved = true;
                    println!("Mouse Move{:?}", self.knob_position);
                }
                cx.request_paint();
            }
            Event::TargetedAccessibilityAction(request) => {
                if request.action == accesskit::Action::Default
                    && cx.is_accesskit_target(request.target)
                {
                    cx.add_message(Message::new(self.id_path.clone(), ()));
                }
            }
            _ => (),
        };
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        if let LifeCycle::HotChanged(_) = event {
            cx.request_paint();
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_layout();
    }

    fn layout(&mut self, _cx: &mut LayoutCx, _bc: &BoxConstraints) -> Size {
        Size::new(SWITCH_WIDTH, SWITCH_HEIGHT)
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        let mut builder = accesskit::NodeBuilder::new(accesskit::Role::Switch);
        builder.set_default_action_verb(accesskit::DefaultActionVerb::Click);
        cx.push_node(builder);
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        // Change the position of of the knob based on its state
        // If the knob is currently being dragged with the mouse use the position that was set in MouseMove
        if !self.is_moved {
            self.knob_position.x = if self.is_on { ON_POS } else { OFF_POS }
        }

        // Paint the Swith background
        // The on/off states have different colors
        // The transition between the two color is controlled by the knob position and calculated using the opacity
        let opacity = (self.knob_position.x - OFF_POS) / (ON_POS - OFF_POS);

        let background_on_state = Color::SPRING_GREEN.with_alpha_factor(opacity as f32);
        let background_off_state = Color::WHITE_SMOKE.with_alpha_factor(1.0 - opacity as f32);

        let background_rect = cx.size().to_rect().to_rounded_rect(SWITCH_HEIGHT / 2.);

        fill_color(builder, &background_rect, background_off_state);
        fill_color(builder, &background_rect, background_on_state);

        // Paint the Switch knob
        println!("Paint: {:?}", self.knob_position);
        let knob_color = if self.is_moved || cx.is_hot() {
            Color::SLATE_GRAY
        } else {
            Color::LIGHT_SLATE_GRAY
        };
        let knob_border_color = Color::DIM_GRAY;
        let mut knob_size = KNOB_DIAMETER / 2.0;

        if cx.is_active() {
            knob_size += 1.0;
        }

        let knob_circle = Circle::new(self.knob_position, knob_size);
        fill_color(builder, &knob_circle, knob_color);
        stroke(builder, &knob_circle, knob_border_color, 2.0);
    }
}
