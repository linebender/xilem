// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::Property;
use resvg::tiny_skia;
use resvg::usvg::Tree;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::peniko::{ImageAlphaType, ImageData, ImageFormat};

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use crate::kurbo::{Affine, Axis, Size};
use crate::layout::LenReq;
use crate::peniko::{BlendMode, Fill, ImageBrush};
use crate::properties::ObjectFit;

// TODO: Make this a configurable option of the widget
/// The scale that the SVG is native to.
///
/// Used to calculate the preferred logical size of the SVG.
///
/// For example you might have a 256 px icon meant for crisp details on 8K screens.
/// Then the SVG could have an preferred scale of e.g. 4.0,
/// meaning that its preferred logical size is 64px.
/// That way the SVG looks good at any scale and doesn't shift the layout around.
const SVG_SCALE: f64 = 1.0;

/// A widget that renders an SVG.
///
/// You can change the sizing of the SVG with the [`ObjectFit`] property.
pub struct Svg {
    tree: Tree,
    rasterized: Option<ImageBrush>,
    decorative: bool,
    alt_text: Option<ArcStr>,
}

// --- MARK: BUILDERS
impl Svg {
    /// Creates an SVG drawing widget.
    ///
    /// By default, the SVG will be scaled to fully fit within the container.
    /// ([`ObjectFit::Contain`]).
    pub fn new(tree: Tree) -> Self {
        Self {
            tree,
            rasterized: None,
            decorative: false,
            alt_text: None,
        }
    }

    /// Specifies whether the SVG is decorative, meaning it doesn't have meaningful content
    /// and is only for visual presentation.
    ///
    /// If `is_decorative` is `true`, the SVG will be ignored by screen readers.
    pub fn decorative(mut self, is_decorative: bool) -> Self {
        self.decorative = is_decorative;
        self
    }

    /// Sets the text that will describe the SVG to screen readers.
    ///
    /// Users are encouraged to set alt text for the SVG.
    /// If possible, the alt-text should succinctly describe what the SVG represents.
    ///
    /// If the SVG is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the SVG content.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: WIDGETMUT
impl Svg {
    /// Sets a new inner SVG.
    pub fn set_tree(this: &mut WidgetMut<'_, Self>, tree: Tree) {
        this.widget.tree = tree;
        this.ctx.request_layout();
    }

    /// Sets whether the SVG is decorative, meaning it doesn't have meaningful content
    /// and is only for visual presentation.
    ///
    /// See [`Svg::decorative`] for details.
    pub fn set_decorative(this: &mut WidgetMut<'_, Self>, is_decorative: bool) {
        this.widget.decorative = is_decorative;
        this.ctx.request_accessibility_update();
    }

    /// Sets the text that will describe the SVG to screen readers.
    ///
    /// See [`Svg::with_alt_text`] for details.
    pub fn set_alt_text(this: &mut WidgetMut<'_, Self>, alt_text: Option<impl Into<ArcStr>>) {
        this.widget.alt_text = alt_text.map(Into::into);
        this.ctx.request_accessibility_update();
    }
}

impl Svg {
    /// Returns the preferred size of the SVG.
    ///
    /// The returned size is in device pixels.
    ///
    /// This takes into account both [`SVG_SCALE`] and `scale`, and so the result
    /// isn't just the SVG data size which would be const across scale factors.
    ///
    /// This method's result will be stable in relation to other widgets at any scale factor.
    ///
    /// Basically it provides logical pixels in device pixel space.
    fn preferred_size(&self, scale: f64) -> Size {
        let size = self.tree.size();
        Size::new(
            size.width() as f64 * scale / SVG_SCALE,
            size.height() as f64 * scale / SVG_SCALE,
        )
    }
}

impl HasProperty<ObjectFit> for Svg {}

// --- MARK: IMPL WIDGET
impl Widget for Svg {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if ObjectFit::matches(property_type) {
            ctx.request_layout();
        }
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
        let preferred_size = self.preferred_size(scale);

        object_fit.measure(axis, len_req, cross_length, preferred_size)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        self.rasterized = None;
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let content_box = ctx.content_box();

        // NOTE: Ideally we would translate the SVG tree into scene draw calls.
        //       However, until Vello API is a thing and Resvg gains that ability,
        //       we'll just have to rasterize it using Tiny Skia and then draw the image.
        if self.rasterized.is_none() {
            let pixmap_width = content_box.width().ceil() as u32;
            let pixmap_height = content_box.height().ceil() as u32;
            if pixmap_width == 0 || pixmap_height == 0 {
                return;
            }
            let mut pixmap = tiny_skia::Pixmap::new(pixmap_width, pixmap_height).unwrap();

            let object_fit = props.get::<ObjectFit>();

            // For drawing we want to scale the actual SVG data lengths, which means
            // we need to avoid using Svg::preferred_size which does not match the data.
            let svg_size = self.tree.size();
            let svg_size = Size::new(svg_size.width() as f64, svg_size.height() as f64);

            let coeffs = object_fit.affine(content_box.size(), svg_size).as_coeffs();
            let transform = tiny_skia::Transform::from_row(
                coeffs[0] as f32,
                coeffs[1] as f32,
                coeffs[2] as f32,
                coeffs[3] as f32,
                coeffs[4] as f32,
                coeffs[5] as f32,
            );

            resvg::render(&self.tree, transform, &mut pixmap.as_mut());

            let image = ImageBrush::new(ImageData {
                data: pixmap.data().to_vec().into(),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::AlphaPremultiplied,
                width: pixmap_width,
                height: pixmap_height,
            });

            self.rasterized = Some(image);
        }

        let image = self.rasterized.as_ref().unwrap();

        scene.push_layer(
            Fill::NonZero,
            BlendMode::default(),
            1.,
            Affine::IDENTITY,
            &content_box,
        );
        scene.draw_image(image, Affine::IDENTITY);
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
        if self.decorative {
            node.set_hidden();
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Svg", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use resvg::usvg;
    use resvg::usvg::fontdb::Database;

    use super::*;
    use crate::testing::{ROBOTO, TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;

    #[test]
    fn empty_tree() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#;
        let tree = Tree::from_str(xml, &usvg::Options::default()).unwrap();
        let svg = Svg::new(tree).with_auto_id();
        let mut harness = TestHarness::create(test_property_set(), svg);
        let _ = harness.render();
    }

    #[test]
    fn brick_wall() {
        let xml = r###"
            <svg xmlns="http://www.w3.org/2000/svg" width="852" height="320" viewBox="0 0 852 320">
                <defs>
                    <!-- Clip the wall so we can reuse full bricks as edge half-bricks -->
                    <clipPath id="wall-clip">
                        <rect x="10" y="10" width="833" height="310" />
                    </clipPath>

                    <!-- One reusable brick -->
                    <g id="brick">
                        <rect x="0" y="0" width="160" height="70" rx="4" ry="4"
                            fill="#b24a3a" stroke="#8b3429" stroke-width="2" />
                        <text x="80" y="42"
                            text-anchor="middle"
                            font-family="Roboto"
                            font-size="24"
                            fill="#f7e7cf">Masonry</text>
                    </g>
                </defs>

                <!-- Mortar background -->
                <rect x="0" y="0" width="852" height="320" fill="#d8d3cd" />

                <!-- All bricks are clipped to the wall area -->
                <g clip-path="url(#wall-clip)">
                    <!-- Row 1 -->
                    <use href="#brick" x="10"  y="10" />
                    <use href="#brick" x="178" y="10" />
                    <use href="#brick" x="346" y="10" />
                    <use href="#brick" x="514" y="10" />
                    <use href="#brick" x="682" y="10" />

                    <!-- Row 2 (staggered + clipped) -->
                    <use href="#brick" x="-70" y="88" />
                    <use href="#brick" x="98"  y="88" />
                    <use href="#brick" x="266" y="88" />
                    <use href="#brick" x="434" y="88" />
                    <use href="#brick" x="602" y="88" />
                    <use href="#brick" x="770" y="88" />

                    <!-- Row 3 -->
                    <use href="#brick" x="10"  y="166" />
                    <use href="#brick" x="178" y="166" />
                    <use href="#brick" x="346" y="166" />
                    <use href="#brick" x="514" y="166" />
                    <use href="#brick" x="682" y="166" />

                    <!-- Row 4 (staggered + clipped) -->
                    <use href="#brick" x="-70" y="244" />
                    <use href="#brick" x="98"  y="244" />
                    <use href="#brick" x="266" y="244" />
                    <use href="#brick" x="434" y="244" />
                    <use href="#brick" x="602" y="244" />
                    <use href="#brick" x="770" y="244" />
                </g>
                </svg>
        "###;

        let mut fontdb = Database::new();
        fontdb.load_font_data(ROBOTO.into());

        let opts = usvg::Options {
            fontdb: Arc::new(fontdb),
            ..usvg::Options::default()
        };

        let tree = Tree::from_str(xml, &opts).unwrap();
        let svg = Svg::new(tree).with_auto_id();

        let window_size = Size::new(852., 320.);
        let mut harness = TestHarness::create_with_size(test_property_set(), svg, window_size);

        assert_render_snapshot!(harness, "svg_brick_wall");
    }
}
