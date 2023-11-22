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

use std::ops::Deref;

use parley::Layout;
use vello::{
    kurbo::{Affine, Insets, Size},
    peniko::{Brush, Color},
    SceneBuilder,
};

use crate::{text::ParleyBrush, IdPath, Message};

use super::{
    contexts::LifeCycleCx,
    piet_scene_helpers::{self, UnitPoint},
    AccessCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle, PaintCx, UpdateCx,
    Widget,
};

pub struct Button {
    id_path: IdPath,
    label: String,
    layout: Option<Layout<ParleyBrush>>,
}

impl Button {
    pub fn new(id_path: &IdPath, label: String) -> Button {
        Button {
            id_path: id_path.clone(),
            label,
            layout: None,
        }
    }

    pub fn set_label(&mut self, label: String) -> ChangeFlags {
        self.label = label;
        self.layout = None;
        ChangeFlags::LAYOUT | ChangeFlags::PAINT
    }
}

// See druid's button for info.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

impl Widget for Button {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        match event {
            Event::MouseDown(_) => {
                cx.set_active(true);
                cx.request_paint();
            }
            Event::MouseUp(_) => {
                if cx.is_hot() && cx.is_active() {
                    cx.add_message(Message::new(self.id_path.clone(), ()));
                }
                cx.set_active(false);
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
            cx.request_paint()
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_layout();
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let min_height = 24.0;
        let mut lcx = parley::LayoutContext::new();
        let mut layout_builder = lcx.ranged_builder(cx.font_cx(), &self.label, 1.0);

        layout_builder.push_default(&parley::style::StyleProperty::Brush(ParleyBrush(
            Brush::Solid(Color::rgb8(0xf0, 0xf0, 0xea)),
        )));
        let mut layout = layout_builder.build();
        // Question for Chad: is this needed?
        layout.break_all_lines(None, parley::layout::Alignment::Start);
        let size = Size::new(
            layout.width() as f64 + padding.width,
            (layout.height() as f64 + padding.height).max(min_height),
        );
        self.layout = Some(layout);
        //(Size::new(10.0, min_height), size)
        cx.request_paint();
        bc.constrain(size)
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        let mut builder = accesskit::NodeBuilder::new(accesskit::Role::Button);
        builder.set_name(self.label.deref());
        builder.set_default_action_verb(accesskit::DefaultActionVerb::Click);
        cx.push_node(builder);
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        let is_hot = cx.is_hot();
        let is_active = cx.is_active();
        let button_border_width = 2.0;
        let rounded_rect = cx
            .size()
            .to_rect()
            .inset(-0.5 * button_border_width)
            .to_rounded_rect(4.0);
        let border_color = if is_hot {
            Color::rgb8(0xa1, 0xa1, 0xa1)
        } else {
            Color::rgb8(0x3a, 0x3a, 0x3a)
        };
        let bg_stops = if is_active {
            [Color::rgb8(0x3a, 0x3a, 0x3a), Color::rgb8(0xa1, 0xa1, 0xa1)]
        } else {
            [Color::rgb8(0xa1, 0xa1, 0xa1), Color::rgb8(0x3a, 0x3a, 0x3a)]
        };
        piet_scene_helpers::stroke(builder, &rounded_rect, border_color, button_border_width);
        piet_scene_helpers::fill_lin_gradient(
            builder,
            &rounded_rect,
            bg_stops,
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
        if let Some(layout) = &self.layout {
            let size = Size::new(layout.width() as f64, layout.height() as f64);
            let offset = (cx.size().to_vec2() - size.to_vec2()) * 0.5;
            let transform = Affine::translate(offset);
            crate::text::render_text(builder, transform, layout);
        }
    }
}
