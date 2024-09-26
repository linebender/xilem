// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{NodeBuilder, Role};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Line, Stroke};
use vello::Scene;

use crate::theme::get_debug_color;
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, Point,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

pub struct Grid {
    children: Vec<Child>,
    grid_width: i32,
    grid_height: i32,
    grid_spacing: f64,
}

struct Child {
    widget: WidgetPod<Box<dyn Widget>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct GridParams {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

// --- MARK: IMPL GRID ---
impl Grid {
    pub fn with_dimensions(width: i32, height: i32) -> Self {
        Grid {
            children: Vec::new(),
            grid_width: width,
            grid_height: height,
            grid_spacing: 0.0,
        }
    }

    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.grid_spacing = spacing;
        self
    }

    /// Builder-style variant of [`WidgetMut::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(self, child: impl Widget, params: GridParams) -> Self {
        self.with_child_pod(WidgetPod::new(Box::new(child)), params)
    }

    pub fn with_child_id(self, child: impl Widget, id: WidgetId, params: GridParams) -> Self {
        self.with_child_pod(WidgetPod::new_with_id(Box::new(child), id), params)
    }

    pub fn with_child_pod(
        mut self,
        widget: WidgetPod<Box<dyn Widget>>,
        params: GridParams,
    ) -> Self {
        let child = Child {
            widget,
            x: params.x,
            y: params.y,
            width: params.width,
            height: params.height,
        };
        self.children.push(child);
        self
    }
}

// --- MARK: IMPL CHILD ---
impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<Box<dyn Widget>>> {
        Some(&mut self.widget)
    }
    fn widget(&self) -> Option<&WidgetPod<Box<dyn Widget>>> {
        Some(&self.widget)
    }

    fn update_params(&mut self, params: GridParams) {
        self.x = params.x;
        self.y = params.y;
        self.width = params.width;
        self.height = params.height;
    }
}

fn new_grid_child(params: GridParams, widget: WidgetPod<Box<dyn Widget>>) -> Child {
    Child {
        widget,
        x: params.x,
        y: params.y,
        width: params.width,
        height: params.height,
    }
}

// --- MARK: IMPL GRIDPARAMS ---
impl GridParams {
    pub fn new(mut x: i32, mut y: i32, mut width: i32, mut height: i32) -> GridParams {
        if x < 0 {
            debug_panic!("Grid x value should be a non-negative number; got {}", x);
            x = 0;
        }
        if y < 0 {
            debug_panic!("Grid y value should be a non-negative number; got {}", y);
            y = 0;
        }
        if width <= 0 {
            debug_panic!(
                "Grid width value should be a positive nonzero number; got {}",
                width
            );
            width = 1;
        }
        if height <= 0 {
            debug_panic!(
                "Grid height value should be a positive nonzero number; got {}",
                height
            );
            height = 1;
        }
        GridParams {
            x,
            y,
            width,
            height,
        }
    }
}

// --- MARK: WIDGETMUT---
impl<'a> WidgetMut<'a, Grid> {
    /// Add a child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Grid::with_child
    pub fn add_child(&mut self, child: impl Widget, params: GridParams) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new(Box::new(child));
        self.insert_child_pod(child_pod, params);
    }

    pub fn add_child_id(&mut self, child: impl Widget, id: WidgetId, params: GridParams) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new_with_id(Box::new(child), id);
        self.insert_child_pod(child_pod, params);
    }

    /// Add a child widget.
    pub fn insert_child_pod(&mut self, widget: WidgetPod<Box<dyn Widget>>, params: GridParams) {
        let child = new_grid_child(params, widget);
        self.widget.children.push(child);
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn insert_grid_child_at(
        &mut self,
        idx: usize,
        child: impl Widget,
        params: impl Into<GridParams>,
    ) {
        self.insert_grid_child_pod(idx, WidgetPod::new(Box::new(child)), params);
    }

    pub fn insert_grid_child_pod(
        &mut self,
        idx: usize,
        child: WidgetPod<Box<dyn Widget>>,
        params: impl Into<GridParams>,
    ) {
        let child = new_grid_child(params.into(), child);
        self.widget.children.insert(idx, child);
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn set_spacing(&mut self, spacing: f64) {
        self.widget.grid_spacing = spacing;
        self.ctx.request_layout();
    }

    pub fn set_width(&mut self, width: i32) {
        self.widget.grid_width = width;
        self.ctx.request_layout();
    }

    pub fn set_height(&mut self, height: i32) {
        self.widget.grid_height = height;
        self.ctx.request_layout();
    }

    pub fn child_mut(&mut self, idx: usize) -> Option<WidgetMut<'_, Box<dyn Widget>>> {
        let child = match self.widget.children[idx].widget_mut() {
            Some(widget) => widget,
            None => return None,
        };

        Some(self.ctx.get_mut(child))
    }

    /// Updates the grid parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_grid_params(&mut self, idx: usize, params: GridParams) {
        let child = &mut self.widget.children[idx];
        child.update_params(params);
        self.ctx.request_layout();
    }

    pub fn remove_child(&mut self, idx: usize) {
        let child = self.widget.children.remove(idx);
        self.ctx.remove_child(child.widget);
        self.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for Grid {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            ctx.register_child(child);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let total_size = bc.max();
        let width_unit = (total_size.width + self.grid_spacing) / (self.grid_width as f64);
        let height_unit = (total_size.height + self.grid_spacing) / (self.grid_height as f64);
        for child in &mut self.children {
            let cell_size = Size::new(
                child.width as f64 * width_unit - self.grid_spacing,
                child.height as f64 * height_unit - self.grid_spacing,
            );
            let child_bc = BoxConstraints::new(cell_size, cell_size);
            let _ = ctx.run_layout(&mut child.widget, &child_bc);
            ctx.place_child(
                &mut child.widget,
                Point::new(child.x as f64 * width_unit, child.y as f64 * height_unit),
            );
        }
        total_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // paint the baseline if we're debugging layout
        if ctx.debug_paint && ctx.widget_state.baseline_offset != 0.0 {
            let color = get_debug_color(ctx.widget_id().to_raw());
            let my_baseline = ctx.size().height - ctx.widget_state.baseline_offset;
            let line = Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Grid")
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::button;

    #[test]
    fn test_grid_basics() {
        // Start with a 1x1 grid
        let widget = Grid::with_dimensions(1, 1)
            .with_child(button::Button::new("A"), GridParams::new(0, 0, 1, 1));
        let mut harness = TestHarness::create(widget);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "initial_1x1");

        // Expand it to a 4x4 grid
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.set_width(4);
        });
        assert_render_snapshot!(harness, "expanded_4x1");

        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.set_height(4);
        });
        assert_render_snapshot!(harness, "expanded_4x4");

        // Add a widget that takes up more than one horizontal cell
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.add_child(button::Button::new("B"), GridParams::new(1, 0, 3, 1));
        });
        assert_render_snapshot!(harness, "with_horizontal_widget");

        // Add a widget that takes up more than one vertical cell
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.add_child(button::Button::new("C"), GridParams::new(0, 1, 1, 3));
        });
        assert_render_snapshot!(harness, "with_vertical_widget");

        // Add a widget that takes up more than one horizontal and vertical cell
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.add_child(button::Button::new("D"), GridParams::new(1, 1, 2, 2));
        });
        assert_render_snapshot!(harness, "with_2x2_widget");

        // Change the spacing
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.set_spacing(7.0);
        });
        assert_render_snapshot!(harness, "with_changed_spacing");

        // Make the spacing negative
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.set_spacing(-4.0);
        });
        assert_render_snapshot!(harness, "with_negative_spacing");
    }

    #[test]
    fn test_widget_removal_and_modification() {
        let widget = Grid::with_dimensions(2, 2)
            .with_child(button::Button::new("A"), GridParams::new(0, 0, 1, 1));
        let mut harness = TestHarness::create(widget);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "initial_2x2");

        // Now remove the widget
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.remove_child(0);
        });
        assert_render_snapshot!(harness, "2x2_with_removed_widget");

        // Add it back
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.add_child(button::Button::new("A"), GridParams::new(0, 0, 1, 1));
        });
        assert_render_snapshot!(harness, "initial_2x2"); // Should be back to the original state

        // Change the grid params to position it on the other corner
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.update_child_grid_params(0, GridParams::new(1, 1, 1, 1));
        });
        assert_render_snapshot!(harness, "moved_2x2_1");

        // Now make it take up the entire grid
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.update_child_grid_params(0, GridParams::new(0, 0, 2, 2));
        });
        assert_render_snapshot!(harness, "moved_2x2_2");
    }

    #[test]
    fn test_widget_order() {
        let widget = Grid::with_dimensions(2, 2)
            .with_child(button::Button::new("A"), GridParams::new(0, 0, 1, 1));
        let mut harness = TestHarness::create(widget);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "initial_2x2");

        // Order sets the draw order, so draw a widget over A by adding it after
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.add_child(button::Button::new("B"), GridParams::new(0, 0, 1, 1));
        });
        assert_render_snapshot!(harness, "2x2_with_overlapping_b");

        // Draw a widget under the others by putting it at index 0
        // Make it wide enough to see it stick out, with half of it under A and B.
        harness.edit_root_widget(|mut grid| {
            let mut grid = grid.downcast::<Grid>();
            grid.insert_grid_child_at(0, button::Button::new("C"), GridParams::new(0, 0, 2, 1));
        });
        assert_render_snapshot!(harness, "2x2_with_overlapping_c");
    }
}
