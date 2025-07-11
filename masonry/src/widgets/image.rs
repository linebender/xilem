// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Size};
use vello::peniko::{BlendMode, Image as ImageBuf};

use crate::core::{
    AccessCtx, BoxConstraints, LayoutCtx, ObjectFit, PaintCtx, PropertiesMut, PropertiesRef,
    QueryCtx, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};

// TODO - Resolve name collision between masonry::Image and peniko::Image

/// A widget that renders a bitmap Image.
pub struct Image {
    image_data: ImageBuf,
    object_fit: ObjectFit,
    alt_text: Option<String>, // ✅ Added for accessibility
}

// --- MARK: BUILDERS
impl Image {
    #[inline]
    pub fn new(image_data: ImageBuf) -> Self {
        Self {
            image_data,
            object_fit: ObjectFit::default(),
            alt_text: None, // ✅ Added default
        }
    }

    #[inline]
    pub fn fit_mode(mut self, mode: ObjectFit) -> Self {
        self.object_fit = mode;
        self
    }

    /// ✅ New method to add alt text
    pub fn with_alt_text(mut self, text: impl Into<String>) -> Self {
        self.alt_text = Some(text.into());
        self
    }
}

// --- MARK: WIDGETMUT
impl Image {
    #[inline]
    pub fn set_fit_mode(this: &mut WidgetMut<'_, Self>, new_object_fit: ObjectFit) {
        this.widget.object_fit = new_object_fit;
        this.ctx.request_layout();
    }

    #[inline]
    pub fn set_image_data(this: &mut WidgetMut<'_, Self>, image_data: ImageBuf) {
        this.widget.image_data = image_data;
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Image {
    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let image_size = Size::new(self.image_data.width as f64, self.image_data.height as f64);
        if image_size.is_zero_area() {
            return bc.min();
        }
        let image_aspect_ratio = image_size.height / image_size.width;
        match self.object_fit {
            ObjectFit::Contain => bc.constrain_aspect_ratio(image_aspect_ratio, image_size.width),
            ObjectFit::Cover => Size::new(bc.max().width, bc.max().width * image_aspect_ratio),
            ObjectFit::Fill => bc.max(),
            ObjectFit::FitHeight => Size::new(bc.max().height / image_aspect_ratio, bc.max().height),
            ObjectFit::FitWidth => Size::new(bc.max().width, bc.max().width * image_aspect_ratio),
            ObjectFit::None => image_size,
            ObjectFit::ScaleDown => {
                let mut size = image_size;
                if !bc.contains(size) {
                    size = bc.constrain_aspect_ratio(image_aspect_ratio, size.width);
                }
                size
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let image_size = Size::new(self.image_data.width as f64, self.image_data.height as f64);
        let transform = self.object_fit.affine_to_fill(ctx.size(), image_size);
        let clip_rect = ctx.size().to_rect();
        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        scene.draw_image(&self.image_data, transform);
        scene.pop_layer();
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        // ✅ Use alt text for accessibility
        if let Some(ref label) = self.alt_text {
            node.set_label(label.clone());
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Image", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use vello::peniko::ImageFormat;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::theme::default_property_set;

    #[test]
    fn empty_paint() {
        let image_data = ImageBuf::new(Vec::new().into(), ImageFormat::Rgba8, 0, 0);
        let image_widget = Image::new(image_data);
        let mut harness = TestHarness::create(default_property_set(), image_widget);
        let _ = harness.render();
    }

    #[test]
    fn tall_paint() {
        let image_data = ImageBuf::new(
            [
                [255, 255, 255, 255],
                [000, 000, 000, 255],
                [000, 000, 000, 255],
                [255, 255, 255, 255],
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into(),
            ImageFormat::Rgba8,
            2,
            2,
        );
        let image_widget = Image::new(image_data);
        let mut harness = TestHarness::create_with_size(
            default_property_set(),
            image_widget,
            Size::new(40., 60.),
        );
        assert_render_snapshot!(harness, "image_tall_paint");
    }

    #[test]
    fn edit_image() {
        let image_data = ImageBuf::new(vec![255; 4 * 8 * 8].into(), ImageFormat::Rgba8, 8, 8);
        let render_1 = {
            let image_widget = Image::new(image_data.clone());
            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                image_widget,
                Size::new(40.0, 60.0),
            );
            harness.render()
        };

        let render_2 = {
            let other_image_data =
                ImageBuf::new(vec![10; 4 * 8 * 8].into(), ImageFormat::Rgba8, 8, 8);
            let image_widget = Image::new(other_image_data);
            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                image_widget,
                Size::new(40.0, 60.0),
            );
            harness.edit_root_widget(|mut image| {
                let mut image = image.downcast::<Image>();
                Image::set_image_data(&mut image, image_data);
            });
            harness.render()
        };

        assert!(render_1 == render_2);
    }

    #[test]
    fn layout() {
        let image_data = ImageBuf::new(vec![255; 4 * 8 * 8].into(), ImageFormat::Rgba8, 8, 8);
        let harness_size = Size::new(100.0, 50.0);

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::Contain);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_contain");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::Cover);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_cover");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::Fill);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_fill");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::FitHeight);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_fitheight");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::FitWidth);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_fitwidth");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::None);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_none");

        let image_widget = Image::new(image_data.clone()).fit_mode(ObjectFit::ScaleDown);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), image_widget, harness_size);
        assert_render_snapshot!(harness, "image_layout_scaledown");
    }
}

