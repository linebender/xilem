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

use glazier::kurbo::{Affine, Insets, RoundedRect, Shape, Size};
use parley::Layout;
use vello::{
    peniko::{Brush, BrushRef, Color},
    SceneBuilder, SceneFragment,
};

use crate::{event::Event, id::IdPath, text::ParleyBrush, VertAlignment};

use super::{
    align::{FirstBaseline, LastBaseline, SingleAlignment},
    contexts::LifeCycleCx,
    piet_scene_helpers::{self, UnitPoint},
    AlignCx, ChangeFlags, EventCx, LayoutCx, LifeCycle, PaintCx, RawEvent, UpdateCx, Widget,
};

pub struct Stroke {
    brush: Brush,
    width: f64,
}

pub struct ButtonStyleState {
    border_radius: f64,
    stroke: Option<Stroke>,
    background_color: Option<Color>,
}

pub struct ButtonStyle {
    default: ButtonStyleState,
    hot: ButtonStyleState,
    active: ButtonStyleState,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        let border_radius = 8.0;
        Self {
            default: ButtonStyleState {
                border_radius,
                stroke: Some(Stroke {
                    brush: Brush::Solid(Color::rgb8(147, 143, 153).into()),
                    width: 2.,
                }),
                background_color: None,
            },
            hot: ButtonStyleState {
                border_radius,
                stroke: None,
                background_color: Some(Color::rgb8(208, 188, 255)),
            },
            active: ButtonStyleState {
                border_radius,
                stroke: None,
                background_color: Some(Color::rgb8(56, 30, 114)),
            },
        }
    }
}

pub struct Button {
    id_path: IdPath,
    label: String,
    layout: Option<Layout<ParleyBrush>>,
    style: ButtonStyle,
}

impl Button {
    pub fn new(id_path: &IdPath, label: String, style: ButtonStyle) -> Button {
        Button {
            id_path: id_path.clone(),
            label,
            layout: None,
            style,
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
    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_layout();
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        match event {
            RawEvent::MouseDown(_) => {
                cx.set_active(true);
                // TODO: request paint
            }
            RawEvent::MouseUp(_) => {
                if cx.is_hot() {
                    cx.add_event(Event::new(self.id_path.clone(), ()));
                }
                cx.set_active(false);
                // TODO: request paint
            }
            _ => (),
        };
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        match event {
            LifeCycle::HotChanged(_) => cx.request_paint(),
            _ => (),
        }
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
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
        (Size::new(10.0, min_height), size)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let size = Size::new(
            proposed_size
                .width
                .clamp(cx.min_size().width, cx.max_size().width),
            cx.max_size().height,
        );
        println!("size = {:?}", size);
        size
    }

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        // TODO: figure this out
        /*
        if alignment.id() == FirstBaseline.id() || alignment.id() == LastBaseline.id() {
            let layout = self.layout.as_ref().unwrap();
            if let Some(metric) = layout.line_metric(0) {
                let value = 0.5 * (cx.size().height - layout.size().height) + metric.baseline;
                cx.aggregate(alignment, value);
            }
        }
        */
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        let is_hot = cx.is_hot();
        let is_active = cx.is_active();

        let style = if is_active {
            &self.style.active
        } else if is_hot {
            &self.style.hot
        } else {
            &self.style.default
        };

        let button_border_width = style
            .stroke
            .as_ref()
            .map(|stroke| stroke.width)
            .unwrap_or(0.);
        let rounded_rect = cx
            .size()
            .to_rect()
            .inset(-0.5 * button_border_width)
            .to_rounded_rect(style.border_radius);

        /*
        let bg_gradient = if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(0x3a, 0x3a, 0x3a), Color::rgb8(0xa1, 0xa1, 0xa1)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(0xa1, 0xa1, 0xa1), Color::rgb8(0x3a, 0x3a, 0x3a)),
            )
        };
        */

        if let Some(stroke) = &style.stroke {
            piet_scene_helpers::stroke(builder, &rounded_rect, &stroke.brush, button_border_width);
        }

        if let Some(bg) = &style.background_color {
            piet_scene_helpers::fill_lin_gradient(
                builder,
                &rounded_rect,
                [bg.clone(), bg.clone()],
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
            );
        }

        /*
        piet_scene_helpers::fill_lin_gradient(
            builder,
            &rounded_rect,
            bg_stops,
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
         */
        //cx.fill(rounded_rect, &bg_gradient);
        if let Some(layout) = &self.layout {
            let size = Size::new(layout.width() as f64, layout.height() as f64);
            let offset = (cx.size().to_vec2() - size.to_vec2()) * 0.5;
            let transform = Affine::translate(offset);
            crate::text::render_text(builder, transform, &layout);
        }
    }
}
