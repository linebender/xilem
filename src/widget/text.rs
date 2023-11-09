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

use parley::{FontContext, Layout};
use std::borrow::Cow;
use vello::{
    kurbo::{Affine, Size},
    peniko::{Brush, Color},
    SceneBuilder,
};

use crate::text::ParleyBrush;

use super::{
    contexts::LifeCycleCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle,
    PaintCx, UpdateCx, Widget,
};

pub struct TextWidget {
    text: Cow<'static, str>,
    layout: Option<Layout<ParleyBrush>>,
}

impl TextWidget {
    pub fn new(text: Cow<'static, str>) -> TextWidget {
        TextWidget { text, layout: None }
    }

    pub fn set_text(&mut self, text: Cow<'static, str>) -> ChangeFlags {
        self.text = text;
        ChangeFlags::LAYOUT | ChangeFlags::PAINT
    }

    fn get_layout_mut(&mut self, font_cx: &mut FontContext) -> &mut Layout<ParleyBrush> {
        // Ensure Parley layout is initialised
        if self.layout.is_none() {
            let mut lcx = parley::LayoutContext::new();
            let mut layout_builder = lcx.ranged_builder(font_cx, self.text.trim(), 1.0);
            layout_builder.push_default(&parley::style::StyleProperty::Brush(ParleyBrush(
                Brush::Solid(Color::rgb8(255, 255, 255)),
            )));
            self.layout = Some(layout_builder.build());
        }

        self.layout.as_mut().unwrap()
    }

    fn layout_text(&mut self, font_cx: &mut FontContext, bc: &BoxConstraints) -> Size {
        // Compute max_advance from box constraints
        let max_advance = if bc.max().width.is_finite() {
            Some(bc.max().width as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };

        // Layout text
        let layout = self.get_layout_mut(font_cx);
        layout.break_all_lines(max_advance, parley::layout::Alignment::Start);

        // // Debug print
        // println!(
        //     "max: {:?}. w: {}, h: {}",
        //     max_advance,
        //     layout.width(),
        //     layout.height()
        // );

        // Return dimensions
        Size {
            width: layout.width() as f64,
            height: layout.height() as f64,
        }
    }
}

impl Widget for TextWidget {
    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut LifeCycleCx, _event: &LifeCycle) {}

    fn update(&mut self, cx: &mut UpdateCx) {
        // All changes potentially require layout. Note: we could be finer
        // grained, maybe color changes wouldn't.
        cx.request_layout();
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: crate::Axis,
        cx: &mut LayoutCx,
        bc: &super::BoxConstraints,
    ) -> f64 {
        let size = self.layout_text(cx.font_cx(), bc);
        match axis {
            crate::Axis::Horizontal => size.width,
            crate::Axis::Vertical => size.height,
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        cx.request_paint();
        self.layout_text(cx.font_cx(), bc)
    }

    fn paint(&mut self, _cx: &mut PaintCx, builder: &mut SceneBuilder) {
        if let Some(layout) = &self.layout {
            crate::text::render_text(builder, Affine::IDENTITY, layout);
        }
    }

    fn accessibility(&mut self, cx: &mut super::AccessCx) {
        let mut builder = accesskit::NodeBuilder::new(accesskit::Role::StaticText);
        builder.set_value(self.text.clone());
        cx.push_node(builder);
    }
}
