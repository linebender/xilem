// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::{CollectionWidget, HasProperty};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Line, Point, Size, Stroke};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Gap, Padding};
use crate::util::{debug_panic, fill, include_screenshot, stroke};

/// A widget that arranges its children in a grid.
///
/// Children are drawn in index order,
/// i.e. each child is drawn on top of the other children with lower indices.
///
#[doc = include_screenshot!("grid_with_changed_spacing.png", "Grid with buttons of various sizes.")]
pub struct Grid {
    children: Vec<Child>,
    grid_column_count: i32,
    grid_row_count: i32,
}

struct Child {
    widget: WidgetPod<dyn Widget>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
/// Parameters required when adding an item to a [`Grid`] container.
pub struct GridParams {
    /// Index of the column this item is starting from.
    pub x: i32,
    /// Index of the row this item is starting from.
    pub y: i32,
    /// Number of columns this item spans.
    pub width: i32,
    /// Number of rows this item spans.
    pub height: i32,
}

// --- MARK: BUILDERS
impl Grid {
    /// Creates a new grid with the given number of columns and rows.
    pub fn with_dimensions(columns: i32, rows: i32) -> Self {
        Self {
            children: Vec::new(),
            grid_column_count: columns,
            grid_row_count: rows,
        }
    }

    /// Builder-style method to add a child widget.
    pub fn with(mut self, child: NewWidget<impl Widget + ?Sized>, params: GridParams) -> Self {
        let child = new_grid_child(params, child);
        self.children.push(child);
        self
    }
}

// --- MARK: IMPL CHILD
impl Child {
    fn update_params(&mut self, params: GridParams) {
        self.x = params.x;
        self.y = params.y;
        self.width = params.width;
        self.height = params.height;
    }
}

fn new_grid_child(params: GridParams, child: NewWidget<impl Widget + ?Sized>) -> Child {
    Child {
        widget: child.erased().to_pod(),
        x: params.x,
        y: params.y,
        width: params.width,
        height: params.height,
    }
}

// --- MARK: IMPL GRIDPARAMS
impl GridParams {
    /// Creates grid parameters with the given values.
    ///
    /// # Panics
    ///
    /// When debug assertions are on, panics if the width or height is less than or equal to zero or if x or y is negative.
    pub fn new(mut x: i32, mut y: i32, mut width: i32, mut height: i32) -> Self {
        // TODO - Use u32 params instead?
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
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

// --- MARK: WIDGETMUT
impl Grid {
    /// Sets the number of columns of the grid.
    pub fn set_column_count(this: &mut WidgetMut<'_, Self>, columns: i32) {
        this.widget.grid_column_count = columns;
        this.ctx.request_layout();
    }

    /// Sets the number of rows of the grid.
    pub fn set_row_count(this: &mut WidgetMut<'_, Self>, rows: i32) {
        this.widget.grid_row_count = rows;
        this.ctx.request_layout();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<GridParams> for Grid {
    /// Returns the number of children.
    fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a mutable reference to the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.children[idx].widget;
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = new_grid_child(params.into(), child);
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Inserts a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = new_grid_child(params.into(), child);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Replaces the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = new_grid_child(params.into(), child);
        let old_child = std::mem::replace(&mut this.widget.children[idx], child);
        this.ctx.remove_child(old_child.widget);
    }

    /// Sets the child params at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<GridParams>) {
        let child = &mut this.widget.children[idx];
        child.update_params(params.into());
        this.ctx.request_layout();
    }

    /// Swaps the index of two children.
    ///
    /// This also swaps the [`GridParams`] `x` and `y` with the other child.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        let (a_x, a_y) = (this.widget.children[a].x, this.widget.children[a].y);
        let (b_x, b_y) = (this.widget.children[b].x, this.widget.children[b].y);

        this.widget.children.swap(a, b);

        (this.widget.children[a].x, this.widget.children[a].y) = (a_x, a_y);
        (this.widget.children[b].x, this.widget.children[b].y) = (b_x, b_y);

        this.ctx.children_changed();
    }

    /// Removes the child at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child.widget);
        }
    }
}

impl HasProperty<Background> for Grid {}
impl HasProperty<BorderColor> for Grid {}
impl HasProperty<BorderWidth> for Grid {}
impl HasProperty<CornerRadius> for Grid {}
impl HasProperty<Padding> for Grid {}
impl HasProperty<Gap> for Grid {}

// --- MARK: IMPL WIDGET
impl Widget for Grid {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut() {
            ctx.register_child(&mut child.widget);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        Gap::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let gap = props.get::<Gap>().gap;

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        let origin = Point::ORIGIN;
        let origin = border.place_down(origin);
        let origin = padding.place_down(origin);
        let origin = origin.to_vec2();

        let total_size = bc.max();
        if !total_size.is_finite() {
            debug_panic!(
                "Error while computing layout for grid; infinite BoxConstraint max provided {}",
                total_size
            );
        }
        let gap = gap.get();
        let width_unit = (total_size.width + gap) / (self.grid_column_count as f64);
        let height_unit = (total_size.height + gap) / (self.grid_row_count as f64);
        for child in &mut self.children {
            let cell_size = Size::new(
                (child.width as f64 * width_unit - gap).max(0.0),
                (child.height as f64 * height_unit - gap).max(0.0),
            );
            let child_bc = BoxConstraints::new(cell_size, cell_size);
            let _ = ctx.run_layout(&mut child.widget, &child_bc);

            let child_pos = Point::new(child.x as f64 * width_unit, child.y as f64 * height_unit);
            ctx.place_child(&mut child.widget, child_pos + origin);
        }

        let (total_size, _) = padding.layout_up(total_size, 0.);
        let (total_size, _) = border.layout_up(total_size, 0.);
        total_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let bg = props.get::<Background>();
        let border_color = props.get::<BorderColor>();

        let bg_rect = border_width.bg_rect(ctx.size(), border_radius);
        let border_rect = border_width.border_rect(ctx.size(), border_radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);

        // paint the baseline if we're debugging layout
        if ctx.debug_paint_enabled() && ctx.baseline_offset() != 0.0 {
            let color = ctx.debug_color();
            let my_baseline = ctx.size().height - ctx.baseline_offset();
            let line = Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        self.children
            .iter()
            .map(|child| child.widget.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Grid", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::types::AsUnit;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Button;

    #[test]
    fn test_grid_basics() {
        // Start with a 1x1 grid
        let widget = NewWidget::new(Grid::with_dimensions(1, 1).with(
            Button::with_text("A").with_auto_id(),
            GridParams::new(0, 0, 1, 1),
        ));
        let window_size = Size::new(200.0, 200.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_1x1");

        // Expand it to a 4x4 grid
        harness.edit_root_widget(|mut grid| {
            Grid::set_column_count(&mut grid, 4);
        });
        assert_render_snapshot!(harness, "grid_expanded_4x1");

        harness.edit_root_widget(|mut grid| {
            Grid::set_row_count(&mut grid, 4);
        });
        assert_render_snapshot!(harness, "grid_expanded_4x4");

        // Add a widget that takes up more than one horizontal cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(
                &mut grid,
                Button::with_text("B").with_auto_id(),
                GridParams::new(1, 0, 3, 1),
            );
        });
        assert_render_snapshot!(harness, "grid_with_horizontal_widget");

        // Add a widget that takes up more than one vertical cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(
                &mut grid,
                Button::with_text("C").with_auto_id(),
                GridParams::new(0, 1, 1, 3),
            );
        });
        assert_render_snapshot!(harness, "grid_with_vertical_widget");

        // Add a widget that takes up more than one horizontal and vertical cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(
                &mut grid,
                Button::with_text("D").with_auto_id(),
                GridParams::new(1, 1, 2, 2),
            );
        });
        assert_render_snapshot!(harness, "grid_with_2x2_widget");

        // Change the gap
        harness.edit_root_widget(|mut grid| {
            grid.insert_prop(Gap::new(7.px()));
        });
        assert_render_snapshot!(harness, "grid_with_changed_spacing");
    }

    #[test]
    fn test_widget_removal_and_modification() {
        let widget = NewWidget::new(Grid::with_dimensions(2, 2).with(
            Button::with_text("A").with_auto_id(),
            GridParams::new(0, 0, 1, 1),
        ));
        let window_size = Size::new(200.0, 200.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_2x2");

        // Now remove the widget
        harness.edit_root_widget(|mut grid| {
            Grid::remove(&mut grid, 0);
        });
        assert_render_snapshot!(harness, "grid_2x2_with_removed_widget");

        // Add it back
        harness.edit_root_widget(|mut grid| {
            Grid::add(
                &mut grid,
                Button::with_text("A").with_auto_id(),
                GridParams::new(0, 0, 1, 1),
            );
        });
        assert_render_snapshot!(harness, "grid_initial_2x2"); // Should be back to the original state

        // Test replacement
        harness.edit_root_widget(|mut grid| {
            Grid::remove(&mut grid, 0);
            Grid::add(
                &mut grid,
                Button::with_text("X").with_auto_id(),
                GridParams::new(0, 0, 1, 1),
            );
        });
        harness.edit_root_widget(|mut grid| {
            Grid::set(
                &mut grid,
                0,
                Button::with_text("A").with_auto_id(),
                GridParams::new(0, 0, 1, 1),
            );
        });
        assert_render_snapshot!(harness, "grid_initial_2x2"); // Should be back to the original state

        // Change the grid params to position it on the other corner
        harness.edit_root_widget(|mut grid| {
            Grid::set_params(&mut grid, 0, GridParams::new(1, 1, 1, 1));
        });
        assert_render_snapshot!(harness, "grid_moved_2x2_1");

        // Now make it take up the entire grid
        harness.edit_root_widget(|mut grid| {
            Grid::set_params(&mut grid, 0, GridParams::new(0, 0, 2, 2));
        });
        assert_render_snapshot!(harness, "grid_moved_2x2_2");
    }

    #[test]
    fn test_widget_order() {
        let widget = NewWidget::new(Grid::with_dimensions(2, 2).with(
            Button::with_text("A").with_auto_id(),
            GridParams::new(0, 0, 1, 1),
        ));
        let window_size = Size::new(200.0, 200.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_2x2");

        // Order sets the draw order, so draw a widget over A by adding it after
        harness.edit_root_widget(|mut grid| {
            Grid::add(
                &mut grid,
                Button::with_text("B").with_auto_id(),
                GridParams::new(0, 0, 1, 1),
            );
        });
        assert_render_snapshot!(harness, "grid_2x2_with_overlapping_b");

        // Draw a widget under the others by putting it at index 0
        // Make it wide enough to see it stick out, with half of it under A and B.
        harness.edit_root_widget(|mut grid| {
            Grid::insert(
                &mut grid,
                0,
                Button::with_text("C").with_auto_id(),
                GridParams::new(0, 0, 2, 1),
            );
        });
        assert_render_snapshot!(harness, "grid_2x2_with_overlapping_c");
    }
}
