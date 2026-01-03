// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A canvas widget.

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, BoxConstraints, ChildrenIds, LayoutCtx, MutateCtx, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut,
};
use crate::kurbo::Size;

/// A widget allowing custom drawing.
///
/// A canvas takes a painter callback; every time the canvas is repainted, that callback
/// in run with a [`Scene`].
/// That Scene is then used as the canvas' contents.
#[derive(Default)]
pub struct Canvas {
    alt_text: Option<ArcStr>,
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
    /// Returns the current size of the canvas
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
        this.widget.scene.reset();
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

    // TODO - Do we want the Canvas to be transparent to pointer events?
    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // We use all the available space as possible.
        let size = bc.max();
        if self.size != size {
            self.size = size;
            ctx.submit_action::<Self::Action>(CanvasSizeChanged { size });
        }

        // We clip the contents we draw.
        ctx.set_clip_path(size.to_rect());

        size
    }

    fn paint(&mut self, _: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        scene.append(&self.scene, None);
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
    use crate::core::{DefaultProperties, Properties};
    use crate::kurbo::{Affine, BezPath, Stroke};
    use crate::peniko::{Color, Fill};
    use crate::testing::TestHarness;

    #[test]
    fn simple_canvas() {
        let canvas = Canvas::default()
            .with_alt_text("A triangle with a bright mint green fill and a gold brown border");

        let mut harness = TestHarness::create(
            DefaultProperties::default(),
            canvas.with_props(Properties::default()),
        );

        harness.edit_root_widget(|mut canvas| {
            Canvas::update_scene(&mut canvas, |_ctx, scene, size| {
                let scale = Affine::scale_non_uniform(size.width, size.height);
                let mut path = BezPath::new();
                path.move_to((0.1, 0.1));
                path.line_to((0.9, 0.9));
                path.line_to((0.9, 0.1));
                path.close_path();
                path = scale * path;
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgb8(100, 240, 150),
                    None,
                    &path,
                );
                scene.stroke(
                    &Stroke::new(4.),
                    Affine::IDENTITY,
                    Color::from_rgb8(200, 140, 50),
                    None,
                    &path,
                );
            });
        });

        assert_render_snapshot!(harness, "canvas_simple");
    }
}
