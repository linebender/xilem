// Copyright 2020 The Druid Authors.
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

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use crate::kurbo::Rect;
use crate::piet::{Image as _, ImageBuf, InterpolationMode, PietImage};
use crate::widget::prelude::*;
use crate::widget::widget_view::WidgetRef;
use crate::widget::FillStrat;

use smallvec::SmallVec;
use tracing::{instrument, trace, trace_span, Span};

/// A widget that renders a bitmap Image.
pub struct Image {
    image_data: ImageBuf,
    paint_data: Option<PietImage>,
    fill: FillStrat,
    interpolation: InterpolationMode,
    clip_area: Option<Rect>,
}

impl Image {
    /// Create an image drawing widget from an image buffer.
    ///
    /// By default, the Image will scale to fit its box constraints ([`FillStrat::Fill`])
    /// and will be scaled bilinearly ([`InterpolationMode::Bilinear`])
    ///
    /// The underlying `ImageBuf` uses `Arc` for buffer data, making it cheap to clone.
    ///
    /// [`FillStrat::Fill`]: crate::widget::FillStrat::Fill
    /// [`InterpolationMode::Bilinear`]: crate::piet::InterpolationMode::Bilinear
    #[inline]
    pub fn new(image_data: ImageBuf) -> Self {
        Image {
            image_data,
            paint_data: None,
            fill: FillStrat::default(),
            interpolation: InterpolationMode::Bilinear,
            clip_area: None,
        }
    }

    /// Builder-style method for specifying the fill strategy.
    #[inline]
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        // Invalidation not necessary
        self
    }

    /// Modify the widget's fill strategy.
    #[inline]
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.fill = newfil;
        // Invalidation not necessary
    }

    /// Builder-style method for specifying the interpolation strategy.
    #[inline]
    pub fn interpolation_mode(mut self, interpolation: InterpolationMode) -> Self {
        self.interpolation = interpolation;
        // Invalidation not necessary
        self
    }

    /// Modify the widget's interpolation mode.
    #[inline]
    pub fn set_interpolation_mode(&mut self, interpolation: InterpolationMode) {
        self.interpolation = interpolation;
        // Invalidation not necessary
    }

    /// Builder-style method for setting the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn clip_area(mut self, clip_area: Option<Rect>) -> Self {
        self.clip_area = clip_area;
        // Invalidation not necessary
        self
    }

    /// Set the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn set_clip_area(&mut self, clip_area: Option<Rect>) {
        self.clip_area = clip_area;
        // Invalidation not necessary
    }

    /// Set new `ImageBuf`.
    #[inline]
    pub fn set_image_data(&mut self, image_data: ImageBuf) {
        self.image_data = image_data;
        self.invalidate();
    }

    /// Invalidate the image cache, forcing it to be recreated.
    #[inline]
    fn invalidate(&mut self) {
        self.paint_data = None;
    }
}

impl Widget for Image {
    fn on_event(&mut self, _ctx: &mut EventCtx, _event: &Event, _env: &Env) {}

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _env: &Env) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _env: &Env) -> Size {
        ctx.init();

        // If either the width or height is constrained calculate a value so that the image fits
        // in the size exactly. If it is unconstrained by both width and height take the size of
        // the image.
        let max = bc.max();
        let image_size = self.image_data.size();
        let size = if bc.is_width_bounded() && !bc.is_height_bounded() {
            let ratio = max.width / image_size.width;
            Size::new(max.width, ratio * image_size.height)
        } else if bc.is_height_bounded() && !bc.is_width_bounded() {
            let ratio = max.height / image_size.height;
            Size::new(ratio * image_size.width, max.height)
        } else {
            bc.constrain(self.image_data.size())
        };
        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _env: &Env) {
        ctx.init();

        let offset_matrix = self.fill.affine_to_fill(ctx.size(), self.image_data.size());

        // The ImageData's to_piet function does not clip to the image's size
        // CairoRenderContext is very like druids but with some extra goodies like clip
        if self.fill != FillStrat::Contain {
            let clip_rect = ctx.size().to_rect();
            ctx.clip(clip_rect);
        }

        let piet_image = {
            let image_data = &self.image_data;
            self.paint_data
                .get_or_insert_with(|| image_data.to_image(ctx.render_ctx))
        };
        if piet_image.size().is_empty() {
            // zero-sized image = nothing to draw
            return;
        }
        ctx.with_save(|ctx| {
            // we have to re-do this because the whole struct is moved into the closure.
            let piet_image = {
                let image_data = &self.image_data;
                self.paint_data
                    .get_or_insert_with(|| image_data.to_image(ctx.render_ctx))
            };
            ctx.transform(offset_matrix);
            if let Some(area) = self.clip_area {
                ctx.draw_image_area(
                    piet_image,
                    area,
                    self.image_data.size().to_rect(),
                    self.interpolation,
                );
            } else {
                ctx.draw_image(
                    piet_image,
                    self.image_data.size().to_rect(),
                    self.interpolation,
                );
            }
        });
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Image")
    }
}
