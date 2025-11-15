// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A canvas widget.

use std::sync::Arc;

use accesskit::{Node, Role};
use masonry_core::core::{ChildrenIds, NoAction};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Size;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut,
};

/// A widget allowing custom drawing.
pub struct Canvas {
    draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>,
    alt_text: Option<String>,
}

// --- MARK: BUILDERS ---
impl Canvas {
    /// Create a new canvas with the given draw function.
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
    /// Users are encouraged to set alt text for the canvas.
    /// If possible, the alt-text should succinctly describe what the canvas represents.
    ///
    /// If the canvas is decorative or too hard to describe through text, users should set alt text to `""`.
    pub fn with_alt_text(mut self, alt_text: impl Into<String>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: WIDGETMUT ---
impl Canvas {
    /// Update the draw function
    pub fn set_painter(
        mut this: &mut WidgetMut<'_, Self>,
        draw: impl Fn(&mut Scene, Size) + Send + Sync + 'static,
    ) {
        Self::set_painter_arc(&mut this, Arc::new(draw));
    }

    /// Update the draw function
    pub fn set_painter_arc(
        this: &mut WidgetMut<'_, Self>,
        draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>,
    ) {
        this.widget.draw = draw;
        this.ctx.request_render();
    }

    /// Set the alternative text for this widget
    pub fn set_alt_text(mut this: WidgetMut<'_, Self>, alt_text: String) {
        this.widget.alt_text = Some(alt_text);
        this.ctx.request_accessibility_update();
    }

    /// Remove the existing alternative text on this widget.
    pub fn remove_alt_text(mut this: WidgetMut<'_, Self>) {
        this.widget.alt_text = None;
        this.ctx.request_accessibility_update();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Canvas {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

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
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // use as much space as possible - caller can size it as necessary
        bc.max()
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
        if let Some(text) = &self.alt_text {
            node.set_description(text.clone());
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, widget_id: WidgetId) -> Span {
        trace_span!("Canvas", id = widget_id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.alt_text.clone()
    }
}

// --- MARK: TESTS ---
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

        let mut harness = TestHarness::create(
            DefaultProperties::default(),
            canvas.with_props(Properties::default()),
        );

        assert_render_snapshot!(harness, "simple_canvas");
    }
}
