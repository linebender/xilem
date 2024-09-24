// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::kurbo::Affine;
use vello::peniko::{BlendMode, Image as ImageBuf};
use vello::Scene;

use crate::widget::{FillStrat, WidgetMut};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

// TODO - Resolve name collision between masonry::Image and peniko::Image

/// A widget that renders a bitmap Image.
///
/// The underlying image uses `Arc` for buffer data, making it cheap to clone.
///
/// This currently uses bilinear interpolation, which falls down when the image is
/// larger than its layout size (e.g. it is in a [sized box](super::SizedBox) smaller
/// than the image size).
pub struct Image {
    image_data: ImageBuf,
    fill: FillStrat,
}

// --- MARK: BUILDERS ---
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

// --- MARK: WIDGETMUT ---
impl<'a> WidgetMut<'a, Image> {
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

// --- MARK: IMPL WIDGET ---
impl Widget for Image {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // If either the width or height is constrained calculate a value so that the image fits
        // in the size exactly. If it is unconstrained by both width and height take the size of
        // the image.
        let image_size = Size::new(self.image_data.width as f64, self.image_data.height as f64);
        if image_size.is_empty() {
            let size = bc.min();
            trace!("Computed size: {}", size);
            return size;
        }
        let image_aspect_ratio = image_size.height / image_size.width;
        let size = match self.fill {
            FillStrat::Contain => bc.constrain_aspect_ratio(image_aspect_ratio, image_size.width),
            FillStrat::Cover => Size::new(bc.max().width, bc.max().width * image_aspect_ratio),
            FillStrat::Fill => bc.max(),
            FillStrat::FitHeight => {
                Size::new(bc.max().height / image_aspect_ratio, bc.max().height)
            }
            FillStrat::FitWidth => Size::new(bc.max().width, bc.max().width * image_aspect_ratio),
            FillStrat::None => image_size,
            FillStrat::ScaleDown => {
                let mut size = image_size;

                if !bc.contains(size) {
                    size = bc.constrain_aspect_ratio(image_aspect_ratio, size.width);
                }

                size
            }
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

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {
        // TODO - Handle alt text and such.
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Image")
    }
}

// FIXME - remove cfg?
#[cfg(not(target_arch = "wasm32"))]
// --- MARK: TESTS ---
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
                let mut image = image.downcast::<Image>();
                image.set_image_data(image_data);
            });

            harness.render()
        };

        // TODO - write comparison function
        // We don't use assert_eq because we don't want rich assert
        assert!(render_1 == render_2);
    }

    #[test]
    fn layout() {
        let image_data = ImageBuf::new(vec![255; 4 * 8 * 8].into(), Format::Rgba8, 8, 8);
        let harness_size = Size::new(100.0, 50.0);

        // Contain.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::Contain);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_contain");

        // Cover.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::Cover);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_cover");

        // Fill.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::Fill);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_fill");

        // FitHeight.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::FitHeight);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_fitheight");

        // FitWidth.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::FitWidth);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_fitwidth");

        // None.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::None);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_none");

        // ScaleDown.
        let image_widget = Image::new(image_data.clone()).fill_mode(FillStrat::ScaleDown);
        let mut harness = TestHarness::create_with_size(image_widget, harness_size);
        assert_render_snapshot!(harness, "layout_scaledown");
    }
}
