// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, LayoutCtx, MeasureCtx, MutateCtx, PaintCtx, PropertiesRef,
    RegisterCtx, Widget, WidgetId, WidgetMut,
};
use crate::imaging::{Painter, record::Scene};
use crate::kurbo::{Axis, Size};
use crate::layout::{LenReq, Length};

/// The preferred size of the square Canvas.
const DEFAULT_LENGTH: Length = Length::const_px(100.);

/// A widget allowing custom drawing.
///
/// A canvas takes a painter callback; every time the canvas is repainted, that callback
/// is run with an `imaging` [`record::Scene`](Scene).
/// That recording is then replayed as the canvas contents.
#[derive(Default)]
pub struct Canvas {
    alt_text: Option<ArcStr>,
    /// The drawable area size, which matches the widget's content-box.
    size: Size,
    scene: Scene,
}

// --- MARK: BUILDERS
impl Canvas {
    /// Sets the text that will describe the canvas to screen readers.
    ///
    /// Users are encouraged to set alt text for the canvas.
    /// If possible, the alt-text should succinctly describe what the canvas represents.
    ///
    /// If the canvas is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the canvas content.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: METHODS
impl Canvas {
    /// Returns the current size of the canvas, which matches its content-box size.
    pub fn size(&self) -> Size {
        self.size
    }
}

// --- MARK: WIDGETMUT
impl Canvas {
    /// Updates the canvas scene.
    pub fn update_scene(
        this: &mut WidgetMut<'_, Self>,
        f: impl FnOnce(&mut MutateCtx<'_>, &mut Scene, Size),
    ) {
        this.widget.scene.clear();
        f(&mut this.ctx, &mut this.widget.scene, this.widget.size);
        this.ctx.request_render();
    }

    /// Sets the text that will describe the canvas to screen readers.
    ///
    /// See [`Canvas::with_alt_text`] for details.
    pub fn set_alt_text(this: &mut WidgetMut<'_, Self>, alt_text: Option<impl Into<ArcStr>>) {
        this.widget.alt_text = alt_text.map(Into::into);
        this.ctx.request_accessibility_update();
    }
}

/// The size of the canvas has changed.
#[derive(Debug)]
pub struct CanvasSizeChanged {
    /// The new size of the canvas
    pub size: Size,
}

// --- MARK: IMPL WIDGET
impl Widget for Canvas {
    type Action = CanvasSizeChanged;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // We use all the available space or fall back to our const preferred size.
        match len_req {
            LenReq::FitContent(space) => space,
            _ => DEFAULT_LENGTH.dp(scale),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        if self.size != size {
            self.size = size;
            ctx.submit_action::<Self::Action>(CanvasSizeChanged { size });
        }
        // We clip the contents we draw.
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        _: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        painter.replay(&self.scene);
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
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

    fn make_trace_span(&self, widget_id: WidgetId) -> Span {
        trace_span!("Canvas", id = widget_id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.alt_text.as_ref().map(ToString::to_string)
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::assert_render_snapshot;

    use super::*;
    use crate::core::{DefaultProperties, PropertySet, render_text};
    use crate::kurbo::{Affine, BezPath, Stroke};
    use crate::parley::{
        Alignment, AlignmentOptions, FontFamily, FontFamilyName, GenericFamily, StyleProperty,
    };
    use crate::peniko::Color;
    use crate::testing::{TestHarness, TestHarnessParams};

    #[test]
    fn simple_canvas() {
        let canvas = Canvas::default()
            .with_alt_text("A triangle with a bright mint green fill and a gold brown border");

        let mut harness = TestHarness::create(
            DefaultProperties::default(),
            canvas.prepare().with_props(PropertySet::default()),
        );

        harness.edit_root_widget(|mut canvas| {
            Canvas::update_scene(&mut canvas, |_ctx, scene, size| {
                let mut painter = Painter::new(scene);
                let scale = Affine::scale_non_uniform(size.width, size.height);
                let mut path = BezPath::new();
                path.move_to((0.1, 0.1));
                path.line_to((0.9, 0.9));
                path.line_to((0.9, 0.1));
                path.close_path();
                path = scale * path;
                painter.fill(&path, Color::from_rgb8(100, 240, 150)).draw();
                painter
                    .stroke(&path, &Stroke::new(4.), Color::from_rgb8(200, 140, 50))
                    .draw();
            });
        });

        assert_render_snapshot!(harness, "canvas_simple");
    }

    #[test]
    fn text_canvas() {
        let canvas =
            Canvas::default().with_alt_text("The text 'Canvas' with a bright mint green fill");

        let harness_params = TestHarnessParams::default().with_size((200, 200));
        let mut harness = TestHarness::create_with(
            DefaultProperties::default(),
            canvas.prepare().with_props(PropertySet::default()),
            harness_params,
        );

        harness.edit_root_widget(|mut canvas| {
            Canvas::update_scene(&mut canvas, |ctx, scene, size| {
                let mut painter = Painter::new(scene);
                let (fcx, lcx) = ctx.text_contexts();
                let mut text_layout_builder = lcx.ranged_builder(fcx, "Canvas", 1., true);
                text_layout_builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
                    FontFamilyName::Generic(GenericFamily::Serif),
                )));
                text_layout_builder.push_default(StyleProperty::FontSize(size.height as f32));
                let mut text_layout = text_layout_builder.build("Canvas");
                text_layout.break_all_lines(None);
                text_layout.align(None, Alignment::Start, AlignmentOptions::default());
                let scale = Affine::scale_non_uniform(
                    size.width / text_layout.width() as f64,
                    size.height / text_layout.height() as f64,
                );
                render_text(
                    &mut painter,
                    scale,
                    &text_layout,
                    &[Color::from_rgb8(100, 240, 150).into()],
                    true,
                );
            });
        });

        assert_render_snapshot!(harness, "canvas_text");
    }
}
