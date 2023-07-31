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

//! Handling of platform and integration events at the widget level.
//!
//! Note: arguably this module should be renamed, perhaps we should use
//! "event" for this level and maybe "message" at the View level.

use glazier::{Modifiers, PointerButton, PointerButtons, PointerType};
use vello::kurbo::{Point, Rect, Vec2};

#[derive(Debug, Clone)]
pub enum Event {
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMove(MouseEvent),
    MouseWheel(MouseEvent),
    MouseLeft(),
    TargetedAccessibilityAction(accesskit::ActionRequest),
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// The position of the mouse in the coordinate space of the receiver.
    pub pos: Point,
    /// The position of the mose in the window coordinate space.
    pub window_pos: Point,
    pub buttons: PointerButtons,
    pub mods: Modifiers,
    pub count: u8,
    pub focus: bool,
    pub button: PointerButton,
    pub wheel_delta: Vec2,
}

#[derive(Debug)]
pub enum LifeCycle {
    HotChanged(bool),
    ViewContextChanged(ViewContext),
    TreeUpdate,
}

#[derive(Debug)]
pub struct ViewContext {
    pub window_origin: Point,
    pub clip: Rect,
    pub mouse_position: Option<Point>,
}

impl<'a> From<&'a glazier::PointerEvent> for MouseEvent {
    fn from(src: &glazier::PointerEvent) -> MouseEvent {
        MouseEvent {
            pos: src.pos,
            window_pos: src.pos,
            buttons: src.buttons,
            mods: src.modifiers,
            count: src.count,
            focus: src.focus,
            button: src.button,
            wheel_delta: if let PointerType::Mouse(ref info) = src.pointer_type {
                info.wheel_delta
            } else {
                Vec2::ZERO
            },
        }
    }
}

impl ViewContext {
    pub fn translate_to(&self, new_origin: Point) -> ViewContext {
        let translate = new_origin.to_vec2();
        ViewContext {
            window_origin: self.window_origin + translate,
            clip: self.clip - translate,
            mouse_position: self.mouse_position.map(|p| p - translate),
        }
    }
}
