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

use vello::{
    kurbo::{Rect, Size},
    peniko::Color,
    SceneBuilder,
};

use super::{
    contexts::LifeCycleCx,
    piet_scene_helpers::{fill_color, stroke},
    AccessCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle, PaintCx, UpdateCx,
    Widget,
};

/// A progress bar, displaying a numeric progress value.
///
/// This type impls `Widget`, expecting a float in the range `0.0..1.0`.
pub struct ProgressBar {
    value: f64,
}

impl ProgressBar {
    pub fn new(value: f64) -> ProgressBar {
        ProgressBar {
            value: value.clamp(0., 1.),
        }
    }

    pub fn set_value(&mut self, value: f64) -> ChangeFlags {
        self.value = value.clamp(0., 1.);
        ChangeFlags::PAINT
    }
}

// See druid's button for info.
const HEIGHT: f64 = 20.0;
const WIDTH: f64 = 200.0;
const STROKE: f64 = 2.0;

impl Widget for ProgressBar {
    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut LifeCycleCx, _event: &LifeCycle) {}

    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_paint();
    }

    fn layout(&mut self, _cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        bc.constrain(Size::new(WIDTH, HEIGHT))
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        let mut builder = accesskit::NodeBuilder::new(accesskit::Role::ProgressIndicator);
        builder.set_default_action_verb(accesskit::DefaultActionVerb::Click);
        cx.push_node(builder);
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        let background_color = Color::WHITE_SMOKE;
        let bar_color = Color::SPRING_GREEN;
        let border_color = Color::DIM_GRAY;

        let progress_background_rect = cx.size().to_rect().to_rounded_rect(HEIGHT / 2.);
        fill_color(builder, &progress_background_rect, background_color);
        stroke(builder, &progress_background_rect, border_color, STROKE);

        println!("{:?}", self.value * cx.size());
        let progress_width = self.value * cx.size().width;
        if progress_width != 0. {
            let progress_bar_rect =
                Rect::from_origin_size((0., 0.), Size::new(progress_width, HEIGHT))
                    .inset(-STROKE / 2.)
                    .to_rounded_rect(HEIGHT / 2.);
            fill_color(builder, &progress_bar_rect, bar_color);
        }

    }
}
