// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! A canvas widget.

use std::sync::Arc;

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::Size;
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};

/// A widget allowing custom drawing.
pub struct Canvas {
    draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>,
    alt_text: Option<String>,
}

// --- MARK: BUILDERS ---
impl Canvas {
    /// Create a new canvas with the given draw function
    pub fn new(draw: impl Fn(&mut Scene, Size) + Send + Sync + 'static) -> Self {
        Self::from_arc(Arc::new(draw))
    }

    /// Create a new canvas from a function already contained in an [`Arc`].
    pub fn from_arc(draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>) -> Self {
        Self {
            draw,
            alt_text: None,
        }
    }

    /// Set the text that will be used to communicate the meaning of the canvas to
    /// those using screen readers.
    ///
    /// Users are strongly encouraged to set alt text for the canvas.
    pub fn with_alt_text(mut self, alt_text: impl Into<String>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: WIDGETMUT ---
impl Canvas {
    /// Update the draw function
    pub fn update_draw(
        this: WidgetMut<'_, Self>,
        draw: impl Fn(&mut Scene, Size) + Send + Sync + 'static,
    ) {
        Self::update_from_arc(this, Arc::new(draw));
    }

    /// Update the draw function
    pub fn update_from_arc(
        mut this: WidgetMut<'_, Self>,
        draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>,
    ) {
        this.widget.draw = draw;
        this.ctx.request_render();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Canvas {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // use as much space as possible - caller can size it as necessary
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        (self.draw)(scene, ctx.size());
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut Node) {
        // TODO: is this correct?
        if let Some(text) = &self.alt_text {
            node.set_description(text.clone());
        } else {
            node.clear_description();
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Canvas", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.alt_text.clone()
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use vello::kurbo::{Affine, BezPath, Stroke};
    use vello::peniko::{Color, Fill};

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;

    #[test]
    fn simple_canvas() {
        let canvas = Canvas::new(|scene, size| {
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

        let mut harness = TestHarness::create(canvas);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello");
    }
}
