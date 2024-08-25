// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::kurbo::Affine;
use vello::peniko::{BlendMode, Color};
use vello::Scene;

use crate::widget::{FillStrat, WidgetMut};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

pub struct Canvas {
    size: Size,
    scene: Scene,
    background_color: Color,
    fill: FillStrat,
}

// --- MARK: BUILDERS ---
impl Canvas {
    /// Create an canvas widget with the given size.
    ///
    /// By default, the canvas' contents will scale to fit its box constraints ([`FillStrat::Fill`]).
    pub fn new(size: Size) -> Self {
        Canvas {
            size,
            scene: Scene::new(),
            background_color: Color::WHITE,
            fill: FillStrat::default(),
        }
    }

    /// Builder-style method for specifying the fill strategy.
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        self
    }

    /// Builder-style method for specifying the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Builder-style method for specifying the initial scene.
    pub fn with_scene(mut self, scene: Scene) -> Self {
        self.scene = scene;
        self
    }
}

// --- MARK: WIDGETMUT ---
impl<'a> WidgetMut<'a, Canvas> {
    /// Modify the widget's fill strategy.
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.widget.fill = newfil;
        self.ctx.request_paint();
    }

    /// Modify the widget's size.
    pub fn set_size(&mut self, size: Size) {
        self.widget.size = size;
        self.ctx.request_layout();
    }

    /// Modify the widget's background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.widget.background_color = color;
        self.ctx.request_paint();
    }

    /// Erase the widget's scene.
    pub fn clear(&mut self) {
        self.widget.scene = Scene::new();
        self.ctx.request_paint();
    }

    /// Erase the widget's scene and replace it with the one provided by the caller.
    pub fn replace_scene(&mut self, scene: Scene) {
        self.widget.scene = scene;
        self.ctx.request_paint();
    }

    /// Get a mutable reference to the widget's scene.
    ///
    /// Mutations to this scene will persist across repaints.
    pub fn scene_mut(&mut self) -> &mut Scene {
        self.ctx.request_paint();
        &mut self.widget.scene
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Canvas {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // If either the width or height is constrained calculate a value so that the canvas fits
        // in the size exactly. If it is unconstrained by both width and height take the size of
        // the canvas.
        let max = bc.max();
        let size = if bc.is_width_bounded() && !bc.is_height_bounded() {
            let ratio = max.width / self.size.width;
            Size::new(max.width, ratio * self.size.height)
        } else if bc.is_height_bounded() && !bc.is_width_bounded() {
            let ratio = max.height / self.size.height;
            Size::new(ratio * self.size.width, max.height)
        } else {
            bc.constrain(self.size)
        };
        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let transform = self.fill.affine_to_fill(ctx.size(), self.size);

        let clip_rect = ctx.size().to_rect();
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            self.background_color,
            None,
            &clip_rect,
        );
        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        scene.append(&self.scene, Some(transform));
        scene.pop_layer();
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {
        // TODO - Handle alt text and such.
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Canvas")
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use vello::kurbo::{Circle, RoundedRect, Stroke};

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;

    const SIZE_1: Size = Size::new(50.0, 50.0);
    const SIZE_2: Size = Size::new(100.0, 100.0);

    fn get_test_scene() -> Scene {
        let mut scene = Scene::new();

        let stroke = Stroke::new(6.0);
        let rect = RoundedRect::new(10.0, 10.0, 80.0, 80.0, 20.0);
        let rect_stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
        scene.stroke(&stroke, Affine::IDENTITY, rect_stroke_color, None, &rect);

        let circle = Circle::new((60.0, 10.0), 40.0);
        let circle_fill_color = Color::rgb(0.9529, 0.5451, 0.6588);
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            circle_fill_color,
            None,
            &circle,
        );

        scene
    }

    /// Painting an empty canvas shouldn't crash.
    #[test]
    fn empty_canvas() {
        let canvas_widget = Canvas::new(SIZE_1);
        let mut harness = TestHarness::create(canvas_widget);
        let _ = harness.render();
    }

    #[test]
    fn simple_scene() {
        let canvas_widget = Canvas::new(SIZE_2).with_scene(get_test_scene());
        let mut harness = TestHarness::create(canvas_widget);

        assert_render_snapshot!(harness, "simple_scene");
    }

    #[test]
    fn edit_canvas() {
        let scene = get_test_scene();

        let render_1 = {
            let canvas_widget = Canvas::new(SIZE_2)
                .with_scene(scene.clone())
                .background_color(Color::GRAY);

            let mut harness = TestHarness::create(canvas_widget);

            harness.render()
        };

        let render_2 = {
            let canvas_widget = Canvas::new(SIZE_1);

            let mut harness = TestHarness::create(canvas_widget);

            harness.edit_root_widget(|mut canvas| {
                let mut canvas = canvas.downcast::<Canvas>();
                canvas.set_size(SIZE_2);
                canvas.set_background_color(Color::GRAY);
                canvas.replace_scene(scene.clone());
            });

            harness.render()
        };

        // TODO - write comparison function
        // We don't use assert_eq because we don't want rich assert
        assert!(render_1 == render_2);
    }
}
