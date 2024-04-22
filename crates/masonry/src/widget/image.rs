// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use kurbo::Affine;
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::peniko::{BlendMode, Image as ImageBuf};
use vello::Scene;

use crate::widget::{FillStrat, WidgetRef};
use crate::{
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent, Size,
    StatusChange, TextEvent, Widget,
};

// TODO - Resolve name collision between masonry::Image and peniko::Image

/// A widget that renders a bitmap Image.
///
/// The underlying image uses `Arc` for buffer data, making it cheap to clone.
pub struct Image {
    image_data: ImageBuf,
    fill: FillStrat,
}

crate::declare_widget!(ImageMut, Image);

impl Image {
    /// Create an image drawing widget from an image buffer.
    ///
    /// By default, the Image will scale to fit its box constraints ([`FillStrat::Fill`]).

    #[inline]
    pub fn new(image_data: ImageBuf) -> Self {
        Image {
            image_data,
            fill: FillStrat::default(),
        }
    }

    /// Builder-style method for specifying the fill strategy.
    #[inline]
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        self
    }
}

impl<'a> ImageMut<'a> {
    /// Modify the widget's fill strategy.
    #[inline]
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.widget.fill = newfil;
        self.ctx.request_paint();
    }

    /// Set new `ImageBuf`.
    #[inline]
    pub fn set_image_data(&mut self, image_data: ImageBuf) {
        self.widget.image_data = image_data;
        self.ctx.request_layout();
    }
}

impl Widget for Image {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // If either the width or height is constrained calculate a value so that the image fits
        // in the size exactly. If it is unconstrained by both width and height take the size of
        // the image.
        let max = bc.max();
        let image_size = Size::new(self.image_data.width as f64, self.image_data.height as f64);
        let size = if bc.is_width_bounded() && !bc.is_height_bounded() {
            let ratio = max.width / image_size.width;
            Size::new(max.width, ratio * image_size.height)
        } else if bc.is_height_bounded() && !bc.is_width_bounded() {
            let ratio = max.height / image_size.height;
            Size::new(ratio * image_size.width, max.height)
        } else {
            bc.constrain(image_size)
        };
        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let image_size = Size::new(self.image_data.width as f64, self.image_data.height as f64);
        let transform = self.fill.affine_to_fill(ctx.size(), image_size);

        let clip_rect = ctx.size().to_rect();
        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        scene.draw_image(&self.image_data, transform);
        scene.pop_layer();
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Image")
    }
}

// FIXME - remove cfg?
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use vello::peniko::Format;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;

    /// Painting an empty image shouldn't crash.
    #[test]
    fn empty_paint() {
        // TODO - Blob::empty() function?
        let image_data = ImageBuf::new(Vec::new().into(), vello::peniko::Format::Rgba8, 0, 0);

        let image_widget = Image::new(image_data);
        let mut harness = TestHarness::create(image_widget);
        let _ = harness.render();
    }

    #[test]
    fn tall_paint() {
        #[rustfmt::skip]
        let image_data = ImageBuf::new(
            vec![
                255, 255, 255, 255, 
                0, 0, 0, 255,
                0, 0, 0, 255,
                255, 255, 255, 255,
            ].into(),
            Format::Rgba8,
            2,
            2,
        );
        let image_widget = Image::new(image_data);

        let mut harness = TestHarness::create_with_size(image_widget, Size::new(40., 60.));
        assert_render_snapshot!(harness, "tall_paint");
    }

    #[test]
    fn edit_image() {
        let image_data = ImageBuf::new(vec![255; 4 * 8 * 8].into(), Format::Rgba8, 8, 8);

        let render_1 = {
            let image_widget = Image::new(image_data.clone());

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.render()
        };

        let render_2 = {
            let other_image_data = ImageBuf::new(vec![10; 4 * 8 * 8].into(), Format::Rgba8, 8, 8);
            let image_widget = Image::new(other_image_data);

            let mut harness = TestHarness::create_with_size(image_widget, Size::new(40.0, 60.0));

            harness.edit_root_widget(|mut image| {
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
