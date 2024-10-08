// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget which splits an area in two, with a settable ratio, and optional draggable resizing.

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, warn, Span};
use vello::Scene;

use crate::dpi::LogicalPosition;
use crate::event::PointerButton;
use crate::kurbo::Line;
use crate::paint_scene_helpers::{fill_color, stroke};
use crate::widget::flex::Axis;
use crate::widget::{WidgetMut, WidgetPod};
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, Color, CursorIcon, EventCtx, LayoutCtx,
    LifeCycleCtx, PaintCtx, Point, PointerEvent, Rect, RegisterCtx, Size, StatusChange, TextEvent,
    Widget, WidgetId,
};

// TODO - Have child widget type as generic argument

/// A container containing two other widgets, splitting the area either horizontally or vertically.
pub struct Split {
    split_axis: Axis,
    split_point_chosen: f64,
    split_point_effective: f64,
    min_size: (f64, f64), // Integers only
    bar_size: f64,        // Integers only
    min_bar_area: f64,    // Integers only
    solid: bool,
    draggable: bool,
    /// The split bar is hovered by the mouse. This state is locked to `true` if the
    /// widget is active (the bar is being dragged) to avoid cursor and painting jitter
    /// if the mouse moves faster than the layout and temporarily gets outside of the
    /// bar area while still being dragged.
    is_bar_hover: bool,
    /// Offset from the split point (bar center) to the actual mouse position when the
    /// bar was clicked. This is used to ensure a click without mouse move is a no-op,
    /// instead of re-centering the bar on the mouse.
    click_offset: f64,
    child1: WidgetPod<Box<dyn Widget>>,
    child2: WidgetPod<Box<dyn Widget>>,
}

// --- MARK: BUILDERS ---
impl Split {
    /// Create a new split panel, with the specified axis being split in two.
    ///
    /// Horizontal split axis means that the children are left and right.
    /// Vertical split axis means that the children are up and down.
    fn new(split_axis: Axis, child1: impl Widget + 'static, child2: impl Widget + 'static) -> Self {
        Split {
            split_axis,
            split_point_chosen: 0.5,
            split_point_effective: 0.5,
            min_size: (0.0, 0.0),
            bar_size: 6.0,
            min_bar_area: 6.0,
            solid: false,
            draggable: false,
            is_bar_hover: false,
            click_offset: 0.0,
            child1: WidgetPod::new(child1).boxed(),
            child2: WidgetPod::new(child2).boxed(),
        }
    }

    /// Create a new split panel, with the horizontal axis split in two by a vertical bar.
    /// The children are laid out left and right.
    pub fn columns(child1: impl Widget + 'static, child2: impl Widget + 'static) -> Self {
        Self::new(Axis::Horizontal, child1, child2)
    }

    /// Create a new split panel, with the vertical axis split in two by a horizontal bar.
    /// The children are laid out up and down.
    pub fn rows(child1: impl Widget + 'static, child2: impl Widget + 'static) -> Self {
        Self::new(Axis::Vertical, child1, child2)
    }

    /// Builder-style method to set the split point as a fraction of the split axis.
    ///
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    pub fn split_point(mut self, split_point: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&split_point),
            "split_point must be in the range [0.0-1.0]!"
        );
        self.split_point_chosen = split_point;
        self
    }

    /// Builder-style method to set the minimum size for both sides of the split axis.
    ///
    /// The value must be greater than or equal to `0.0`.
    /// The value will be rounded up to the nearest integer.
    pub fn min_size(mut self, first: f64, second: f64) -> Self {
        assert!(first >= 0.0);
        assert!(second >= 0.0);
        self.min_size = (first.ceil(), second.ceil());
        self
    }

    /// Builder-style method to set the size of the splitter bar.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    pub fn bar_size(mut self, bar_size: f64) -> Self {
        assert!(bar_size >= 0.0, "bar_size must be 0.0 or greater!");
        self.bar_size = bar_size.ceil();
        self
    }

    /// Builder-style method to set the minimum size of the splitter bar area.
    ///
    /// The minimum splitter bar area defines the minimum size of the area
    /// where mouse hit detection is done for the splitter bar.
    /// The final area is either this or the splitter bar size, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    pub fn min_bar_area(mut self, min_bar_area: f64) -> Self {
        assert!(min_bar_area >= 0.0, "min_bar_area must be 0.0 or greater!");
        self.min_bar_area = min_bar_area.ceil();
        self
    }

    /// Builder-style method to set whether the split point can be changed by dragging.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Builder-style method to set whether the splitter bar is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), the bar will be drawn as two parallel lines.
    pub fn solid_bar(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }
}

// --- MARK: INTERNALS ---
impl Split {
    /// Returns the size of the splitter bar area.
    #[inline]
    fn bar_area(&self) -> f64 {
        self.bar_size.max(self.min_bar_area)
    }

    /// Returns the padding size added to each side of the splitter bar.
    #[inline]
    fn bar_padding(&self) -> f64 {
        (self.bar_area() - self.bar_size) / 2.0
    }

    /// Returns the position of the split point (split bar center).
    fn bar_position(&self, size: Size) -> f64 {
        let bar_area = self.bar_area();
        match self.split_axis {
            Axis::Horizontal => {
                let reduced_width = size.width - bar_area;
                let edge1 = (reduced_width * self.split_point_effective).floor();
                edge1 + bar_area / 2.0
            }
            Axis::Vertical => {
                let reduced_height = size.height - bar_area;
                let edge1 = (reduced_height * self.split_point_effective).floor();
                edge1 + bar_area / 2.0
            }
        }
    }

    /// Returns the location of the edges of the splitter bar area,
    /// given the specified total size.
    fn bar_edges(&self, size: Size) -> (f64, f64) {
        let bar_area = self.bar_area();
        match self.split_axis {
            Axis::Horizontal => {
                let reduced_width = size.width - bar_area;
                let edge1 = (reduced_width * self.split_point_effective).floor();
                let edge2 = edge1 + bar_area;
                (edge1, edge2)
            }
            Axis::Vertical => {
                let reduced_height = size.height - bar_area;
                let edge1 = (reduced_height * self.split_point_effective).floor();
                let edge2 = edge1 + bar_area;
                (edge1, edge2)
            }
        }
    }

    /// Returns true if the provided mouse position is inside the splitter bar area.
    fn bar_hit_test(&self, size: Size, mouse_pos: LogicalPosition<f64>) -> bool {
        let (edge1, edge2) = self.bar_edges(size);
        match self.split_axis {
            Axis::Horizontal => mouse_pos.x >= edge1 && mouse_pos.x <= edge2,
            Axis::Vertical => mouse_pos.y >= edge1 && mouse_pos.y <= edge2,
        }
    }

    /// Returns the minimum and maximum split coordinate of the provided size.
    fn split_side_limits(&self, size: Size) -> (f64, f64) {
        let split_axis_size = self.split_axis.major(size);

        let (mut min_limit, min_second) = self.min_size;
        let mut max_limit = (split_axis_size - min_second).max(0.0);

        if min_limit > max_limit {
            min_limit = 0.5 * (min_limit + max_limit);
            max_limit = min_limit;
        }

        (min_limit, max_limit)
    }

    /// Set a new chosen split point.
    fn update_split_point(&mut self, size: Size, mouse_pos: Point) {
        let (min_limit, max_limit) = self.split_side_limits(size);
        self.split_point_chosen = match self.split_axis {
            Axis::Horizontal => mouse_pos.x.clamp(min_limit, max_limit) / size.width,
            Axis::Vertical => mouse_pos.y.clamp(min_limit, max_limit) / size.height,
        }
    }

    /// Returns the color of the splitter bar.
    fn bar_color(&self) -> Color {
        if self.draggable {
            theme::BORDER_LIGHT
        } else {
            theme::BORDER_DARK
        }
    }

    fn paint_solid_bar(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();
        let (edge1, edge2) = self.bar_edges(size);
        let padding = self.bar_padding();
        let rect = match self.split_axis {
            Axis::Horizontal => Rect::from_points(
                Point::new(edge1 + padding.ceil(), 0.0),
                Point::new(edge2 - padding.floor(), size.height),
            ),
            Axis::Vertical => Rect::from_points(
                Point::new(0.0, edge1 + padding.ceil()),
                Point::new(size.width, edge2 - padding.floor()),
            ),
        };
        let splitter_color = self.bar_color();
        fill_color(scene, &rect, splitter_color);
    }

    fn paint_stroked_bar(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();
        // Set the line width to a third of the splitter bar size,
        // because we'll paint two equal lines at the edges.
        let line_width = (self.bar_size / 3.0).floor();
        let line_midpoint = line_width / 2.0;
        let (edge1, edge2) = self.bar_edges(size);
        let padding = self.bar_padding();
        let (line1, line2) = match self.split_axis {
            Axis::Horizontal => (
                Line::new(
                    Point::new(edge1 + line_midpoint + padding.ceil(), 0.0),
                    Point::new(edge1 + line_midpoint + padding.ceil(), size.height),
                ),
                Line::new(
                    Point::new(edge2 - line_midpoint - padding.floor(), 0.0),
                    Point::new(edge2 - line_midpoint - padding.floor(), size.height),
                ),
            ),
            Axis::Vertical => (
                Line::new(
                    Point::new(0.0, edge1 + line_midpoint + padding.ceil()),
                    Point::new(size.width, edge1 + line_midpoint + padding.ceil()),
                ),
                Line::new(
                    Point::new(0.0, edge2 - line_midpoint - padding.floor()),
                    Point::new(size.width, edge2 - line_midpoint - padding.floor()),
                ),
            ),
        };
        let splitter_color = self.bar_color();
        stroke(scene, &line1, splitter_color, line_width);
        stroke(scene, &line2, splitter_color, line_width);
    }
}

// FIXME - Add unit tests for WidgetMut<Split>

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, Split> {
    /// Set the split point as a fraction of the split axis.
    ///
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    pub fn set_split_point(&mut self, split_point: f64) {
        assert!(
            (0.0..=1.0).contains(&split_point),
            "split_point must be in the range [0.0-1.0]!"
        );
        self.widget.split_point_chosen = split_point;
        self.ctx.request_layout();
    }

    /// Set the minimum size for both sides of the split axis.
    ///
    /// The value must be greater than or equal to `0.0`.
    /// The value will be rounded up to the nearest integer.
    pub fn set_min_size(&mut self, first: f64, second: f64) {
        assert!(first >= 0.0);
        assert!(second >= 0.0);
        self.widget.min_size = (first.ceil(), second.ceil());
        self.ctx.request_layout();
    }

    /// Set the size of the splitter bar.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    pub fn set_bar_size(&mut self, bar_size: f64) {
        assert!(bar_size >= 0.0, "bar_size must be 0.0 or greater!");
        self.widget.bar_size = bar_size.ceil();
        self.ctx.request_layout();
    }

    /// Set the minimum size of the splitter bar area.
    ///
    /// The minimum splitter bar area defines the minimum size of the area
    /// where mouse hit detection is done for the splitter bar.
    /// The final area is either this or the splitter bar size, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    pub fn set_min_bar_area(&mut self, min_bar_area: f64) {
        assert!(min_bar_area >= 0.0, "min_bar_area must be 0.0 or greater!");
        self.widget.min_bar_area = min_bar_area.ceil();
        self.ctx.request_layout();
    }

    /// Set whether the split point can be changed by dragging.
    pub fn set_draggable(&mut self, draggable: bool) {
        self.widget.draggable = draggable;
        self.ctx.request_paint();
    }

    /// Set whether the splitter bar is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), the bar will be drawn as two parallel lines.
    pub fn set_bar_solid(&mut self, solid: bool) {
        self.widget.solid = solid;
        self.ctx.request_paint();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Split {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        if self.draggable {
            match event {
                PointerEvent::PointerDown(PointerButton::Primary, state) => {
                    if self.bar_hit_test(ctx.size(), state.position) {
                        ctx.set_handled();
                        ctx.capture_pointer();
                        // Save the delta between the mouse click position and the split point
                        self.click_offset = match self.split_axis {
                            Axis::Horizontal => state.position.x,
                            Axis::Vertical => state.position.y,
                        } - self.bar_position(ctx.size());
                        // If not already hovering, force and change cursor appropriately
                        if !self.is_bar_hover {
                            self.is_bar_hover = true;
                            match self.split_axis {
                                Axis::Horizontal => ctx.set_cursor(&CursorIcon::EwResize),
                                Axis::Vertical => ctx.set_cursor(&CursorIcon::NsResize),
                            };
                        }
                    }
                }
                PointerEvent::PointerUp(PointerButton::Primary, state) => {
                    if ctx.has_pointer_capture() {
                        ctx.set_handled();
                        // Depending on where the mouse cursor is when the button is released,
                        // the cursor might or might not need to be changed
                        self.is_bar_hover =
                            ctx.is_hot() && self.bar_hit_test(ctx.size(), state.position);
                        if !self.is_bar_hover {
                            ctx.clear_cursor();
                        }
                    }
                }
                PointerEvent::PointerMove(state) => {
                    if ctx.has_pointer_capture() {
                        // If active, assume always hover/hot
                        let effective_pos = match self.split_axis {
                            Axis::Horizontal => {
                                Point::new(state.position.x - self.click_offset, state.position.y)
                            }
                            Axis::Vertical => {
                                Point::new(state.position.x, state.position.y - self.click_offset)
                            }
                        };
                        self.update_split_point(ctx.size(), effective_pos);
                        ctx.request_layout();
                    } else {
                        // If not active, set cursor when hovering state changes
                        let hover = ctx.is_hot() && self.bar_hit_test(ctx.size(), state.position);
                        if self.is_bar_hover != hover {
                            self.is_bar_hover = hover;
                            if hover {
                                match self.split_axis {
                                    Axis::Horizontal => ctx.set_cursor(&CursorIcon::EwResize),
                                    Axis::Vertical => ctx.set_cursor(&CursorIcon::NsResize),
                                };
                            } else {
                                ctx.clear_cursor();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.child1);
        ctx.register_child(&mut self.child2);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        match self.split_axis {
            Axis::Horizontal => {
                if !bc.is_width_bounded() {
                    warn!("A Split widget was given an unbounded width to split.");
                }
            }
            Axis::Vertical => {
                if !bc.is_height_bounded() {
                    warn!("A Split widget was given an unbounded height to split.");
                }
            }
        }

        let mut my_size = bc.max();
        let bar_area = self.bar_area();
        let reduced_size = Size::new(
            (my_size.width - bar_area).max(0.),
            (my_size.height - bar_area).max(0.),
        );

        // Update our effective split point to respect our constraints
        self.split_point_effective = {
            let (min_limit, max_limit) = self.split_side_limits(reduced_size);
            let reduced_axis_size = self.split_axis.major(reduced_size);
            if reduced_axis_size.is_infinite() || reduced_axis_size <= f64::EPSILON {
                0.5
            } else {
                self.split_point_chosen
                    .clamp(min_limit / reduced_axis_size, max_limit / reduced_axis_size)
            }
        };

        // TODO - The minimum height / width should really be zero here.

        let (child1_bc, child2_bc) = match self.split_axis {
            Axis::Horizontal => {
                let child1_width = (reduced_size.width * self.split_point_effective)
                    .floor()
                    .max(0.0);
                let child2_width = (reduced_size.width - child1_width).max(0.0);
                (
                    BoxConstraints::new(Size::new(child1_width, bc.max().height)),
                    BoxConstraints::new(Size::new(child2_width, bc.max().height)),
                )
            }
            Axis::Vertical => {
                let child1_height = (reduced_size.height * self.split_point_effective)
                    .floor()
                    .max(0.0);
                let child2_height = (reduced_size.height - child1_height).max(0.0);
                (
                    BoxConstraints::new(Size::new(bc.max().width, child1_height)),
                    BoxConstraints::new(Size::new(bc.max().width, child2_height)),
                )
            }
        };

        let child1_size = ctx.run_layout(&mut self.child1, &child1_bc);
        let child2_size = ctx.run_layout(&mut self.child2, &child2_bc);

        // Top-left align for both children, out of laziness.
        // Reduce our unsplit direction to the larger of the two widgets
        let child1_pos = Point::ORIGIN;
        let child2_pos = match self.split_axis {
            Axis::Horizontal => {
                my_size.height = child1_size.height.max(child2_size.height);
                Point::new(child1_size.width + bar_area, 0.0)
            }
            Axis::Vertical => {
                my_size.width = child1_size.width.max(child2_size.width);
                Point::new(0.0, child1_size.height + bar_area)
            }
        };
        ctx.place_child(&mut self.child1, child1_pos);
        ctx.place_child(&mut self.child2, child2_pos);

        let child1_paint_rect = ctx.child_paint_rect(&self.child1);
        let child2_paint_rect = ctx.child_paint_rect(&self.child2);
        let paint_rect = child1_paint_rect.union(child2_paint_rect);
        let insets = paint_rect - my_size.to_rect();
        ctx.set_paint_insets(insets);

        trace!("Computed layout: size={}, insets={:?}", my_size, insets);
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // TODO - Paint differently if the bar is draggable and hovered.
        if self.solid {
            self.paint_solid_bar(ctx, scene);
        } else {
            self.paint_stroked_bar(ctx, scene);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Splitter
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.child1.id(), self.child2.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Split")
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Label;

    #[test]
    fn columns() {
        #[rustfmt::skip]
        let widget = Split::columns(
            Label::new("Hello"),
            Label::new("World"),
        );

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "columns");
    }

    #[test]
    fn rows() {
        #[rustfmt::skip]
        let widget = Split::rows(
            Label::new("Hello"),
            Label::new("World"),
        );

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "rows");
    }

    // FIXME - test moving the split point by mouse
    // test draggable and min_bar_area

    #[test]
    fn edit_splitter() {
        let image_1 = {
            let widget = Split::rows(Label::new("Hello"), Label::new("World"))
                .split_point(0.3)
                .min_size(40.0, 10.0)
                .bar_size(12.0)
                .draggable(true)
                .solid_bar(true);

            let mut harness = TestHarness::create_with_size(widget, Size::new(100.0, 100.0));

            harness.render()
        };

        let image_2 = {
            let widget = Split::rows(Label::new("Hello"), Label::new("World"));

            let mut harness = TestHarness::create_with_size(widget, Size::new(100.0, 100.0));

            harness.edit_root_widget(|mut splitter| {
                let mut splitter = splitter.downcast::<Split>();

                splitter.set_split_point(0.3);
                splitter.set_min_size(40.0, 10.0);
                splitter.set_bar_size(12.0);
                splitter.set_draggable(true);
                splitter.set_bar_solid(true);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
