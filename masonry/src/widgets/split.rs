// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget which splits an area in two, with a settable ratio, and optional draggable resizing.

use accesskit::{Node, Role};
use tracing::{Span, trace_span, warn};
use ui_events::pointer::PointerButtonEvent;
use vello::Scene;
use vello::kurbo::{Line, Point, Rect, Size};

use crate::core::{
    AccessCtx, AccessEvent, Axis, BoxConstraints, ChildrenIds, CursorIcon, EventCtx, FromDynWidget,
    LayoutCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, TextEvent, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::peniko::Color;
use crate::properties::types::{AsUnit, Length};
use crate::theme;
use crate::util::{fill_color, include_screenshot, stroke};

// TODO - Remove size rounding.
// Pixel snapping is now done at the Masonry level.

/// A container containing two other widgets, splitting the area either horizontally or vertically.
///
#[doc = include_screenshot!("split_columns.png", "Split panel with two labels.")]
pub struct Split<ChildA, ChildB>
where
    ChildA: Widget + ?Sized,
    ChildB: Widget + ?Sized,
{
    split_axis: Axis,
    split_point_chosen: f64,
    split_point_effective: f64,
    min_size: (Length, Length), // Integers only
    bar_size: Length,           // Integers only
    min_bar_area: Length,       // Integers only
    solid: bool,
    draggable: bool,
    /// Offset from the split point (bar center) to the actual mouse position when the
    /// bar was clicked. This is used to ensure a click without mouse move is a no-op,
    /// instead of re-centering the bar on the mouse.
    click_offset: f64,
    child1: WidgetPod<ChildA>,
    child2: WidgetPod<ChildB>,
}

// --- MARK: BUILDERS
impl<ChildA: Widget + ?Sized, ChildB: Widget + ?Sized> Split<ChildA, ChildB> {
    /// Create a new split panel.
    pub fn new(child1: NewWidget<ChildA>, child2: NewWidget<ChildB>) -> Self {
        Self {
            split_axis: Axis::Horizontal,
            split_point_chosen: 0.5,
            split_point_effective: 0.5,
            min_size: (Length::ZERO, Length::ZERO),
            bar_size: 6.px(),
            min_bar_area: 6.px(),
            solid: false,
            draggable: true,
            click_offset: 0.0,
            child1: child1.to_pod(),
            child2: child2.to_pod(),
        }
    }

    /// Builder-style method to set the split axis.
    ///
    /// Horizontal split axis means that the children are left and right.
    /// Vertical split axis means that the children are up and down.
    ///
    /// The default split point is horizontal.
    pub fn split_axis(mut self, split_axis: Axis) -> Self {
        self.split_axis = split_axis;
        self
    }

    /// Builder-style method to set the split point as a fraction of the split axis.
    ///
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    pub fn split_point(mut self, split_point: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&split_point),
            "split_point must be in the range [0.0, 1.0], got {split_point}"
        );
        self.split_point_chosen = split_point;
        self
    }

    /// Builder-style method to set the minimum size for both sides of the split axis.
    ///
    /// The value will be rounded up to the nearest integer.
    pub fn min_size(mut self, first: Length, second: Length) -> Self {
        self.min_size = (ceil_length(first), ceil_length(second));
        self
    }

    /// Builder-style method to set the size of the splitter bar.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    pub fn bar_size(mut self, bar_size: Length) -> Self {
        self.bar_size = ceil_length(bar_size);
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
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    pub fn min_bar_area(mut self, min_bar_area: Length) -> Self {
        self.min_bar_area = ceil_length(min_bar_area);
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

// --- MARK: INTERNALS
// TODO - Remove this function, and remove pixel-snapping code from this file.
#[doc(hidden)]
pub fn ceil_length(l: Length) -> Length {
    Length::px(l.get().ceil())
}

impl<ChildA: Widget + ?Sized, ChildB: Widget + ?Sized> Split<ChildA, ChildB> {
    /// Returns the size of the splitter bar area.
    #[inline]
    fn bar_area(&self) -> f64 {
        self.bar_size.get().max(self.min_bar_area.get())
    }

    /// Returns the padding size added to each side of the splitter bar.
    #[inline]
    fn bar_padding(&self) -> f64 {
        (self.bar_area() - self.bar_size.get()) / 2.0
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

    /// Returns `true` if the provided mouse position is inside the splitter bar area.
    fn bar_hit_test(&self, size: Size, mouse_pos: Point) -> bool {
        let (edge1, edge2) = self.bar_edges(size);
        match self.split_axis {
            Axis::Horizontal => mouse_pos.x >= edge1 && mouse_pos.x <= edge2,
            Axis::Vertical => mouse_pos.y >= edge1 && mouse_pos.y <= edge2,
        }
    }

    /// Returns the minimum and maximum split coordinate of the provided size.
    fn split_side_limits(&self, size: Size) -> (f64, f64) {
        let split_axis_size = self.split_axis.major(size);

        let (min_limit, min_second) = self.min_size;
        let mut min_limit = min_limit.get();
        let min_second = min_second.get();
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
            theme::ZYNC_500
        } else {
            theme::ZYNC_700
        }
    }

    fn paint_solid_bar(&mut self, ctx: &mut PaintCtx<'_>, scene: &mut Scene) {
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

    fn paint_stroked_bar(&mut self, ctx: &mut PaintCtx<'_>, scene: &mut Scene) {
        let size = ctx.size();
        // Set the line width to a third of the splitter bar size,
        // because we'll paint two equal lines at the edges.
        let line_width = (self.bar_size.get() / 3.0).floor();
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

// --- MARK: WIDGETMUT
impl<ChildA, ChildB> Split<ChildA, ChildB>
where
    ChildA: Widget + FromDynWidget + ?Sized,
    ChildB: Widget + FromDynWidget + ?Sized,
{
    /// Get a mutable reference to the first child widget.
    pub fn child1_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, ChildA> {
        this.ctx.get_mut(&mut this.widget.child1)
    }

    /// Get a mutable reference to the second child widget.
    pub fn child2_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, ChildB> {
        this.ctx.get_mut(&mut this.widget.child2)
    }

    /// Set the split axis.
    pub fn set_split_axis(this: &mut WidgetMut<'_, Self>, split_axis: Axis) {
        this.widget.split_axis = split_axis;
        this.ctx.request_layout();
    }

    /// Set the split point as a fraction of the split axis.
    ///
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    pub fn set_split_point(this: &mut WidgetMut<'_, Self>, split_point: f64) {
        assert!(
            (0.0..=1.0).contains(&split_point),
            "split_point must be in the range [0.0-1.0]!"
        );
        this.widget.split_point_chosen = split_point;
        this.ctx.request_layout();
    }

    /// Set the minimum size for both sides of the split axis.
    ///
    /// The value will be rounded up to the nearest integer.
    pub fn set_min_size(this: &mut WidgetMut<'_, Self>, first: Length, second: Length) {
        this.widget.min_size = (ceil_length(first), ceil_length(second));
        this.ctx.request_layout();
    }

    /// Set the size of the splitter bar.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    pub fn set_bar_size(this: &mut WidgetMut<'_, Self>, bar_size: Length) {
        this.widget.bar_size = ceil_length(bar_size);
        this.ctx.request_layout();
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
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    pub fn set_min_bar_area(this: &mut WidgetMut<'_, Self>, min_bar_area: Length) {
        this.widget.min_bar_area = ceil_length(min_bar_area);
        this.ctx.request_layout();
    }

    /// Set whether the split point can be changed by dragging.
    pub fn set_draggable(this: &mut WidgetMut<'_, Self>, draggable: bool) {
        this.widget.draggable = draggable;
        // Bar mutability impacts appearance, but not accessibility node
        // TODO - This might change in a future implementation
        this.ctx.request_paint_only();
    }

    /// Set whether the splitter bar is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), the bar will be drawn as two parallel lines.
    pub fn set_bar_solid(this: &mut WidgetMut<'_, Self>, solid: bool) {
        this.widget.solid = solid;
        // Bar solidity impacts appearance, but not accessibility node
        this.ctx.request_paint_only();
    }
}

// --- MARK: IMPL WIDGET
impl<ChildA, ChildB> Widget for Split<ChildA, ChildB>
where
    ChildA: Widget + ?Sized,
    ChildB: Widget + ?Sized,
{
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.draggable {
            match event {
                PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                    let pos = ctx.local_position(state.position);
                    if self.bar_hit_test(ctx.size(), pos) {
                        ctx.set_handled();
                        ctx.capture_pointer();
                        // Save the delta between the mouse click position and the split point
                        self.click_offset = match self.split_axis {
                            Axis::Horizontal => pos.x,
                            Axis::Vertical => pos.y,
                        } - self.bar_position(ctx.size());
                    }
                }
                PointerEvent::Move(u) => {
                    if ctx.is_active() {
                        let pos = ctx.local_position(u.current.position);
                        // If widget has pointer capture, assume always it's hovered
                        let effective_pos = match self.split_axis {
                            Axis::Horizontal => Point {
                                x: pos.x - self.click_offset,
                                y: pos.y,
                            },
                            Axis::Vertical => Point {
                                x: pos.x,
                                y: pos.y - self.click_offset,
                            },
                        };
                        self.update_split_point(ctx.size(), effective_pos);
                        ctx.request_layout();
                    }
                }
                _ => {}
            }
        }
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

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child1);
        ctx.register_child(&mut self.child2);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
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
                    BoxConstraints::new(
                        Size::new(child1_width, bc.min().height),
                        Size::new(child1_width, bc.max().height),
                    ),
                    BoxConstraints::new(
                        Size::new(child2_width, bc.min().height),
                        Size::new(child2_width, bc.max().height),
                    ),
                )
            }
            Axis::Vertical => {
                let child1_height = (reduced_size.height * self.split_point_effective)
                    .floor()
                    .max(0.0);
                let child2_height = (reduced_size.height - child1_height).max(0.0);
                (
                    BoxConstraints::new(
                        Size::new(bc.min().width, child1_height),
                        Size::new(bc.max().width, child1_height),
                    ),
                    BoxConstraints::new(
                        Size::new(bc.min().width, child2_height),
                        Size::new(bc.max().width, child2_height),
                    ),
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

        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO - Paint differently if the bar is draggable and hovered.
        if self.solid {
            self.paint_solid_bar(ctx, scene);
        } else {
            self.paint_stroked_bar(ctx, scene);
        }
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        let local_mouse_pos = pos - ctx.window_origin().to_vec2();
        let is_bar_hovered = self.bar_hit_test(ctx.size(), local_mouse_pos);

        if self.draggable && (ctx.is_active() || is_bar_hovered) {
            match self.split_axis {
                Axis::Horizontal => CursorIcon::EwResize,
                Axis::Vertical => CursorIcon::NsResize,
            }
        } else {
            CursorIcon::Default
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Splitter
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child1.id(), self.child2.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Split", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::Label;

    #[test]
    fn columns() {
        #[rustfmt::skip]
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        ).split_axis(Axis::Horizontal).draggable(false).with_auto_id();

        let window_size = Size::new(150.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "split_columns");
    }

    #[test]
    fn rows() {
        #[rustfmt::skip]
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        ).split_axis(Axis::Vertical).draggable(false).with_auto_id();

        let window_size = Size::new(150.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "split_rows");
    }

    // FIXME - test moving the split point by mouse
    // test draggable and min_bar_area

    #[test]
    fn edit_splitter() {
        let image_1 = {
            let widget = Split::new(
                Label::new("Hello").with_auto_id(),
                Label::new("World").with_auto_id(),
            )
            .split_point(0.3)
            .min_size(40.px(), 10.px())
            .bar_size(12.px())
            .draggable(true)
            .solid_bar(true)
            .with_auto_id();

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                widget,
                Size::new(100.0, 100.0),
            );

            harness.render()
        };

        let image_2 = {
            let widget = Split::new(
                Label::new("Hello").with_auto_id(),
                Label::new("World").with_auto_id(),
            )
            .with_auto_id();

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                widget,
                Size::new(100.0, 100.0),
            );

            harness.edit_root_widget(|mut splitter| {
                Split::set_split_point(&mut splitter, 0.3);
                Split::set_min_size(&mut splitter, 40.px(), 10.px());
                Split::set_bar_size(&mut splitter, 12.px());
                Split::set_draggable(&mut splitter, true);
                Split::set_bar_solid(&mut splitter, true);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
