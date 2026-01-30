// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use crate::kurbo::{Affine, Axis, Size};
use crate::layout::LenReq;
use crate::peniko::{BlendMode, Fill, ImageBrush};
use crate::properties::ObjectFit;

// TODO: Make this a configurable option of the widget.
/// The scale that the image is native to.
///
/// Used to calculate the preferred logical size of the image.
///
/// For example you might have a 256 px icon meant for crisp details on 8K screens.
/// Then the image could have an preferred scale of e.g. 4.0,
/// meaning that its preferred logical size is 64px.
/// That way the image looks good at any scale and doesn't shift the layout around.
const IMAGE_SCALE: f64 = 1.0;

// TODO - Resolve name collision between masonry::Image and peniko::Image

/// A widget that renders a bitmap Image.
///
/// The underlying image uses `Arc` for buffer data, making it cheap to clone.
///
/// This currently uses bilinear interpolation, which falls down when the image is
/// larger than its layout size (e.g. it is in a [sized box](super::SizedBox) smaller
/// than the image size).
///
/// You can change the sizing of the image with the [`ObjectFit`] property.
pub struct Image {
    image_data: ImageBrush,
    alt_text: Option<ArcStr>,
}

// --- MARK: BUILDERS
impl Image {
    /// Creates an image drawing widget from an image buffer.
    ///
    /// By default, the image will be scaled to fully fit within the container.
    /// ([`ObjectFit::Contain`]).
    #[inline]
    pub fn new(image_data: impl Into<ImageBrush>) -> Self {
        Self {
            image_data: image_data.into(),
            alt_text: None,
        }
    }

    /// Sets the text that will describe the image to screen readers.
    ///
    /// Users are encouraged to set alt text for the image.
    /// If possible, the alt-text should succinctly describe what the image represents.
    ///
    /// If the image is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the image content.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: WIDGETMUT
impl Image {
    /// Sets new `ImageBrush`.
    #[inline]
    pub fn set_image_data(this: &mut WidgetMut<'_, Self>, image_data: impl Into<ImageBrush>) {
        this.widget.image_data = image_data.into();
        this.ctx.request_layout();
    }

    /// Sets the text that will describe the image to screen readers.
    ///
    /// See [`Image::with_alt_text`] for details.
    pub fn set_alt_text(this: &mut WidgetMut<'_, Self>, alt_text: Option<impl Into<ArcStr>>) {
        this.widget.alt_text = alt_text.map(Into::into);
        this.ctx.request_accessibility_update();
    }
}

impl Image {
    /// Returns the preferred length of the given `axis`.
    ///
    /// The returned length is in device pixels.
    ///
    /// This takes into account both [`IMAGE_SCALE`] and `scale`, and so the result
    /// isn't just the image data length which would be const across scale factors.
    ///
    /// This method's result will be stable in relation to other widgets at any scale factor.
    ///
    /// Basically it provides logical pixels in device pixel space.
    fn preferred_length(&self, axis: Axis, scale: f64) -> f64 {
        match axis {
            Axis::Horizontal => self.image_data.image.width as f64 * scale / IMAGE_SCALE,
            Axis::Vertical => self.image_data.image.height as f64 * scale / IMAGE_SCALE,
        }
    }
}

impl HasProperty<ObjectFit> for Image {}

// --- MARK: IMPL WIDGET
impl Widget for Image {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        ObjectFit::prop_changed(ctx, property_type);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let object_fit = props.get::<ObjectFit>();

        let ar = {
            let (numerator, denominator) = match axis {
                Axis::Horizontal => (self.image_data.image.width, self.image_data.image.height),
                Axis::Vertical => (self.image_data.image.height, self.image_data.image.width),
            };
            if denominator > 0 {
                numerator as f64 / denominator as f64
            } else {
                1.
            }
        };

        let preferred_length = self.preferred_length(axis, scale);
        let (space, space_or_preferred) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (f64::INFINITY, preferred_length),
            LenReq::FitContent(space) => (space, space),
        };

        match object_fit {
            // Use all available space or if cross is known attempt to maintain AR,
            // but don't exceed available space (will letterbox cross).
            ObjectFit::Contain => cross_length
                .map(|cl| (cl * ar).min(space))
                .unwrap_or(space_or_preferred),
            // Always use all available space.
            ObjectFit::Cover | ObjectFit::Stretch => space_or_preferred,
            // Always use all available vertical space.
            // Greedily take all horizontal space unless cross is known.
            ObjectFit::FitHeight => match axis {
                Axis::Horizontal => cross_length
                    .map(|cl| (cl * ar).min(space))
                    .unwrap_or(space_or_preferred),
                Axis::Vertical => space_or_preferred,
            },
            // Always use all available horizontal space.
            // Greedily take all vertical space unless cross is known.
            ObjectFit::FitWidth => match axis {
                Axis::Horizontal => space_or_preferred,
                Axis::Vertical => cross_length
                    .map(|cl| (cl * ar).min(space))
                    .unwrap_or(space_or_preferred),
            },
            // None == preferred size
            ObjectFit::None => preferred_length,
            // ScaleDown == min(Contain, None)
            ObjectFit::ScaleDown => cross_length
                .map(|cl| (cl * ar).min(space))
                .unwrap_or(space_or_preferred)
                .min(preferred_length),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let object_fit = props.get::<ObjectFit>();
        // For drawing we want to scale the actual image data lengths, which means
        // we need to avoid using Image::preferred_length which does not match the data.
        let image_size = Size::new(
            self.image_data.image.width as f64,
            self.image_data.image.height as f64,
        );
        let transform = object_fit.affine(ctx.size(), image_size);

        let clip_rect = ctx.size().to_rect();
        scene.push_layer(
            Fill::NonZero,
            BlendMode::default(),
            1.,
            Affine::IDENTITY,
            &clip_rect,
        );
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
        if let Some(alt_text) = &self.alt_text {
            node.set_description(&**alt_text);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Image", id = id.trace())
    }
}

// FIXME - remove cfg?
#[cfg(not(target_arch = "wasm32"))]
// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::NewWidget;
    use crate::peniko::{ImageAlphaType, ImageData, ImageFormat};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;

    /// Painting an empty image shouldn't crash.
    #[test]
    fn empty_paint() {
        // TODO - Blob::empty() function?
        // TODO: Does Vello promise this is supported?
        let image_data = ImageData {
            data: Vec::new().into(),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: 0,
            height: 0,
        };

        let image_widget = NewWidget::new(Image::new(image_data));
        let mut harness = TestHarness::create(test_property_set(), image_widget);
        let _ = harness.render();
    }

    #[test]
    fn tall_paint() {
        let image_data = ImageData {
            // This could have a more concise chain, but previously used versions either
            // had unreadable formatting or used `rustfmt::skip`, which broke formatting
            // across large parts of the file.
            data: [
                [255, 255, 255, 255],
                [000, 000, 000, 255],
                [000, 000, 000, 255],
                [255, 255, 255, 255],
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into(),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: 2,
            height: 2,
        };
        let image_widget = NewWidget::new(Image::new(image_data));

        let mut harness =
            TestHarness::create_with_size(test_property_set(), image_widget, Size::new(40., 60.));
        assert_render_snapshot!(harness, "image_tall_paint");
    }

    #[test]
    fn edit_image() {
        let image_data = ImageData {
            data: vec![255; 4 * 8 * 8].into(),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: 8,
            height: 8,
        };

        let render_1 = {
            let image_widget = NewWidget::new(Image::new(image_data.clone()));

            let mut harness = TestHarness::create_with_size(
                test_property_set(),
                image_widget,
                Size::new(40.0, 60.0),
            );

            harness.render()
        };

        let render_2 = {
            let other_image_data = ImageData {
                data: vec![10; 4 * 8 * 8].into(),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::Alpha,
                width: 8,
                height: 8,
            };
            let image_widget = NewWidget::new(Image::new(other_image_data));

            let mut harness = TestHarness::create_with_size(
                test_property_set(),
                image_widget,
                Size::new(40.0, 60.0),
            );

            harness.edit_root_widget(|mut image| {
                Image::set_image_data(&mut image, image_data);
            });

            harness.render()
        };

        // TODO - Use Kompari instead
        // We don't use assert_eq because we don't want rich assert
        assert!(render_1 == render_2);
    }

    #[test]
    fn layout() {
        let image_data = ImageData {
            data: vec![255; 4 * 8 * 8].into(),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: 8,
            height: 8,
        };
        let harness_size = Size::new(100.0, 50.0);

        let image_widget = NewWidget::new(Image::new(image_data.clone()));
        let mut harness =
            TestHarness::create_with_size(test_property_set(), image_widget, harness_size);

        // Contain.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::Contain);
        });
        assert_render_snapshot!(harness, "image_layout_contain");

        // Cover.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::Cover);
        });
        assert_render_snapshot!(harness, "image_layout_cover");

        // FitHeight.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::FitHeight);
        });
        assert_render_snapshot!(harness, "image_layout_fitheight");

        // FitWidth.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::FitWidth);
        });
        assert_render_snapshot!(harness, "image_layout_fitwidth");

        // None.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::None);
        });
        assert_render_snapshot!(harness, "image_layout_none");

        // ScaleDown.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::ScaleDown);
        });
        assert_render_snapshot!(harness, "image_layout_scaledown");

        // Stretch.
        harness.edit_root_widget(|mut image| {
            image.insert_prop(ObjectFit::Stretch);
        });
        assert_render_snapshot!(harness, "image_layout_stretch");
    }
}
