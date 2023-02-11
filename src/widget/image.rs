// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

use crate::kurbo::Rect;
use crate::piet::{Image as _, ImageBuf, InterpolationMode, PietImage};
use crate::widget::{FillStrat, WidgetRef};
use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, StatusChange, Widget,
};

/// A widget that renders a bitmap Image.
pub struct Image {
    image_data: ImageBuf,
    paint_data: Option<PietImage>,
    fill: FillStrat,
    interpolation: InterpolationMode,
    clip_area: Option<Rect>,
}

crate::declare_widget!(ImageMut, Image);

impl Image {
    /// Create an image drawing widget from an image buffer.
    ///
    /// By default, the Image will scale to fit its box constraints ([`FillStrat::Fill`])
    /// and will be scaled bilinearly ([`InterpolationMode::Bilinear`])
    ///
    /// The underlying `ImageBuf` uses `Arc` for buffer data, making it cheap to clone.
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
        self
    }

    /// Builder-style method for specifying the interpolation strategy.
    #[inline]
    pub fn interpolation_mode(mut self, interpolation: InterpolationMode) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Builder-style method for setting the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn clip_area(mut self, clip_area: Option<Rect>) -> Self {
        self.clip_area = clip_area;
        self
    }
}

impl<'a, 'b> ImageMut<'a, 'b> {
    /// Modify the widget's fill strategy.
    #[inline]
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.widget.fill = newfil;
        self.ctx.request_paint();
    }

    /// Modify the widget's interpolation mode.
    #[inline]
    pub fn set_interpolation_mode(&mut self, interpolation: InterpolationMode) {
        self.widget.interpolation = interpolation;
        self.ctx.request_paint();
    }

    /// Set the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn set_clip_area(&mut self, clip_area: Option<Rect>) {
        self.widget.clip_area = clip_area;
        self.ctx.request_paint();
    }

    /// Set new `ImageBuf`.
    #[inline]
    pub fn set_image_data(&mut self, image_data: ImageBuf) {
        self.widget.image_data = image_data;
        self.widget.paint_data = None;
        self.ctx.request_layout();
    }
}

impl Widget for Image {
    fn on_event(&mut self, _ctx: &mut EventCtx, _event: &Event, _env: &Env) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _env: &Env) -> Size {
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
        let offset_matrix = self.fill.affine_to_fill(ctx.size(), self.image_data.size());

        // The ImageData's to_piet function does not clip to the image's size
        // CairoRenderContext is very like Masonry's but with some extra goodies like clip
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

#[allow(unused)]
// FIXME - remove cfg?
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::piet::ImageFormat;
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt};
    use crate::theme::PRIMARY_LIGHT;

    /// Painting an empty image shouldn't crash.
    #[test]
    fn empty_paint() {
        let image_data = ImageBuf::empty();

        let image_widget =
            Image::new(image_data).interpolation_mode(InterpolationMode::NearestNeighbor);

        let mut harness = TestHarness::create(image_widget);
        let _ = harness.render();
    }

    #[test]
    fn tall_paint() {
        let image_data = ImageBuf::from_raw(
            vec![255, 255, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255],
            ImageFormat::Rgb,
            2,
            2,
        );

        let image_widget =
            Image::new(image_data).interpolation_mode(InterpolationMode::NearestNeighbor);

        let mut harness = TestHarness::create_with_size(image_widget, Size::new(40., 60.));
        assert_render_snapshot!(harness, "tall_paint");
    }

    #[test]
    fn edit_image_attributes() {
        let image_data = ImageBuf::from_raw(
            vec![
                255, 255, 255, 244, 244, 244, 255, 255, 255, // row 0
                0, 0, 0, 1, 1, 1, 0, 0, 0, // row 1
                255, 255, 255, 244, 244, 244, 255, 255, 255, // row 2
            ],
            ImageFormat::Rgb,
            3,
            3,
        );

        let render_1 = {
            let image_widget = Image::new(image_data.clone())
                .fill_mode(FillStrat::Cover)
                .interpolation_mode(InterpolationMode::NearestNeighbor)
                .clip_area(Some(Rect::new(0.0, 0.0, 1.0, 1.0)));

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.render()
        };

        let render_2 = {
            let image_widget = Image::new(image_data);

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.edit_root_widget(|mut image, _| {
                let mut image = image.downcast::<Image>().unwrap();
                image.set_fill_mode(FillStrat::Cover);
                image.set_interpolation_mode(InterpolationMode::NearestNeighbor);
                image.set_clip_area(Some(Rect::new(0.0, 0.0, 1.0, 1.0)));
            });

            harness.render()
        };

        // TODO - write comparison function that creates rich diff
        // and saves it in /tmp folder - See issue #18
        // We don't use assert_eq because we don't want rich assert
        assert!(render_1 == render_2);
    }

    #[test]
    fn edit_image() {
        let image_data = ImageBuf::from_raw(vec![255; 3 * 8 * 8], ImageFormat::Rgb, 8, 8);

        let render_1 = {
            let image_widget = Image::new(image_data.clone());

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.render()
        };

        let render_2 = {
            let other_image_data = ImageBuf::from_raw(vec![10; 3 * 8 * 8], ImageFormat::Rgb, 8, 8);
            let image_widget = Image::new(other_image_data);

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.edit_root_widget(|mut image, _| {
                let mut image = image.downcast::<Image>().unwrap();
                image.set_image_data(image_data);
            });

            harness.render()
        };

        // TODO - write comparison function
        // We don't use assert_eq because we don't want rich assert
        assert!(render_1 == render_2);
    }
}
