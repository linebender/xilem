// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A canvas widget.

use accesskit::{Node, Role};
use masonry_core::core::{ArcStr, ChildrenIds, NoAction};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Size;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut,
};

// TODO - Add background color?

/// A widget allowing custom drawing.
///
/// A canvas takes a painter callback; every time the canvas is repainted, that callback
/// in run with a [`Scene`].
/// That Scene is then used as the canvas' contents.
pub struct Canvas {
    draw: fn(&mut Scene, Size),
    alt_text: ArcStr,
}

// --- MARK: BUILDERS
impl Canvas {
    /// Create a new canvas from a function already contained in an [`Arc`].
    pub fn new(draw: fn(&mut Scene, Size), alt_text: impl Into<ArcStr>) -> Self {
        Self {
            draw,
            alt_text: alt_text.into(),
        }
    }

    /// Set the text that will describe the canvas to screen readers.
    ///
    /// Users are encouraged to set alt text for the canvas.
    /// If possible, the alt-text should succinctly describe what the canvas represents.
    ///
    /// If the canvas is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the canvas content.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = alt_text.into();
        self
    }
}

// --- MARK: WIDGETMUT
impl Canvas {
    /// Update the draw function.
    pub fn set_draw(this: &mut WidgetMut<'_, Self>, draw: fn(&mut Scene, Size)) {
        this.widget.draw = draw;
        this.ctx.request_render();
    }

    /// Set the text that will describe the canvas to screen readers.
    ///
    /// See [`Canvas::with_alt_text`] for details.
    pub fn set_alt_text(mut this: WidgetMut<'_, Self>, alt_text: impl Into<ArcStr>) {
        this.widget.alt_text = alt_text.into();
        this.ctx.request_accessibility_update();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Canvas {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    // TODO - Do we want the Canvas to be transparent to pointer events?
    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

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
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // We use all the available space as possible.
        let size = bc.max();

        // We clip the contents we draw.
        ctx.set_clip_path(size.to_rect());

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        (self.draw)(scene, ctx.size());
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
        if !self.alt_text.is_empty() {
            node.set_description(&*self.alt_text);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, widget_id: WidgetId) -> Span {
        trace_span!("Canvas", id = widget_id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.alt_text.to_string())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_core::core::{DefaultProperties, Properties};
    use masonry_testing::assert_render_snapshot;
    use vello::kurbo::{Affine, BezPath, Stroke};
    use vello::peniko::{Color, Fill};

    use super::*;
    use crate::testing::TestHarness;

    #[test]
    fn simple_canvas() {
        let canvas = Canvas::new(
            |scene, size| {
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
            },
            "A triangle with a bright mint green fill and a gold brown border",
        );

        let mut harness = TestHarness::create(
            DefaultProperties::default(),
            canvas.with_props(Properties::default()),
        );

        assert_render_snapshot!(harness, "canvas_simple");
    }
}
