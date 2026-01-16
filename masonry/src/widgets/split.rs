// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget which splits an area in two, with a settable ratio, and optional draggable resizing.

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, CursorIcon, EventCtx, FromDynWidget, LayoutCtx,
    MeasureCtx, NewWidget, NoAction, PaintCtx, PointerButtonEvent, PointerEvent, PointerUpdate,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::kurbo::{Axis, Line, Point, Rect, Size};
use crate::layout::{AsUnit, LayoutSize, LenReq, Length};
use crate::peniko::Color;
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
    min_lengths: (Length, Length), // Integers only
    bar_thickness: Length,         // Integers only
    min_bar_area: Length,          // Integers only
    solid: bool,
    draggable: bool,
    /// Offset from the split point (bar center) to the actual position where the
    /// bar was clicked. This is used to ensure a click without moving is a no-op,
    /// instead of re-centering the bar to the click position.
    click_offset: f64,
    child1: WidgetPod<ChildA>,
    child2: WidgetPod<ChildB>,
}

// --- MARK: BUILDERS
impl<ChildA: Widget + ?Sized, ChildB: Widget + ?Sized> Split<ChildA, ChildB> {
    /// Creates a new split panel.
    pub fn new(child1: NewWidget<ChildA>, child2: NewWidget<ChildB>) -> Self {
        Self {
            split_axis: Axis::Horizontal,
            split_point_chosen: 0.5,
            split_point_effective: 0.5,
            min_lengths: (Length::ZERO, Length::ZERO),
            bar_thickness: 6.px(),
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

    /// Builder-style method to set the minimum length for both sides of the split axis.
    ///
    /// The values will be rounded up to the nearest integer.
    pub fn min_lengths(mut self, first: Length, second: Length) -> Self {
        self.min_lengths = (ceil_length(first), ceil_length(second));
        self
    }

    /// Builder-style method to set the thickness of the splitter bar.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar thickness is `6.0`.
    pub fn bar_thickness(mut self, bar_thickness: Length) -> Self {
        self.bar_thickness = ceil_length(bar_thickness);
        self
    }

    /// Builder-style method to set the minimum thickness of the splitter bar area.
    ///
    /// The minimum splitter bar area defines the minimum thickness of the area
    /// where pointer hit detection is done for the splitter bar.
    /// The final hit detection area thickness is either this minimum
    /// or the splitter bar thickness, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area thickness is `6.0`.
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

// TODO - Remove this function, and remove pixel-snapping code from this file.
#[doc(hidden)]
pub fn ceil_length(l: Length) -> Length {
    Length::px(l.get().ceil())
}

// --- MARK: METHODS
impl<ChildA: Widget + ?Sized, ChildB: Widget + ?Sized> Split<ChildA, ChildB> {
    /// Returns the thickness of the splitter bar area.
    #[inline]
    fn bar_area(&self, scale: f64) -> f64 {
        self.bar_thickness.max(self.min_bar_area).dp(scale)
    }

    /// Returns the padding added to each side of the splitter bar on the split axis.
    #[inline]
    fn bar_padding(&self, scale: f64) -> f64 {
        (self.bar_area(scale) - self.bar_thickness.dp(scale)) * 0.5
    }

    /// Returns the position of the split point (splitter bar area center).
    fn bar_position(&self, length: f64, scale: f64) -> f64 {
        let bar_area = self.bar_area(scale);
        let reduced_length = length - bar_area;
        let edge = (reduced_length * self.split_point_effective).floor();
        edge + bar_area * 0.5
    }

    /// Returns the location of the edges of the splitter bar area,
    /// given the specified total length.
    fn bar_edges(&self, length: f64, scale: f64) -> (f64, f64) {
        let bar_area = self.bar_area(scale);
        let reduced_length = length - bar_area;
        let edge = (reduced_length * self.split_point_effective).floor();
        (edge, edge + bar_area)
    }

    /// Returns `true` if the provided position is on the splitter bar area.
    fn bar_hit_test(&self, length: f64, pos: f64, scale: f64) -> bool {
        let (edge1, edge2) = self.bar_edges(length, scale);
        pos >= edge1 && pos <= edge2
    }

    /// Returns the minimum and maximum split coordinate of the provided length.
    fn split_side_limits(&self, length: f64, scale: f64) -> (f64, f64) {
        let (min_limit, min_second) = self.min_lengths;
        let mut min_limit = min_limit.dp(scale);
        let min_second = min_second.dp(scale);
        let mut max_limit = (length - min_second).max(0.0);

        if min_limit > max_limit {
            min_limit = 0.5 * (min_limit + max_limit);
            max_limit = min_limit;
        }

        (min_limit, max_limit)
    }

    fn calc_effective_split_point(&self, length: f64, scale: f64) -> f64 {
        let (min_limit, max_limit) = self.split_side_limits(length, scale);
        if length <= f64::EPSILON {
            0.5
        } else {
            self.split_point_chosen
                .clamp(min_limit / length, max_limit / length)
        }
    }

    /// Sets a new chosen split point.
    fn update_split_point(&mut self, length: f64, pos: f64, scale: f64) {
        let (min_limit, max_limit) = self.split_side_limits(length, scale);
        self.split_point_chosen = pos.clamp(min_limit, max_limit) / length;
    }

    /// Returns the color of the splitter bar.
    fn bar_color(&self) -> Color {
        if self.draggable {
            theme::ZYNC_500
        } else {
            theme::ZYNC_700
        }
    }

    fn paint_solid_bar(&mut self, ctx: &mut PaintCtx<'_>, scene: &mut Scene, scale: f64) {
        let size = ctx.size();
        let length = size.get_coord(self.split_axis);
        let cross_length = size.get_coord(self.split_axis.cross());
        let (edge1, edge2) = self.bar_edges(length, scale);
        let padding = self.bar_padding(scale);

        let p1 = self.split_axis.pack_point(edge1 + padding.ceil(), 0.);
        let p2 = self
            .split_axis
            .pack_point(edge2 - padding.floor(), cross_length);
        let rect = Rect::from_points(p1, p2);

        let splitter_color = self.bar_color();
        fill_color(scene, &rect, splitter_color);
    }

    fn paint_stroked_bar(&mut self, ctx: &mut PaintCtx<'_>, scene: &mut Scene, scale: f64) {
        let size = ctx.size();
        let length = size.get_coord(self.split_axis);
        let cross_length = size.get_coord(self.split_axis.cross());
        // Set the line width to a third of the splitter bar thickness,
        // because we'll paint two equal lines at the edges.
        let line_width = (self.bar_thickness.dp(scale) / 3.0).floor();
        let line_midpoint = line_width / 2.0;
        let (edge1, edge2) = self.bar_edges(length, scale);
        let padding = self.bar_padding(scale);

        let edge1_line_pos = edge1 + line_midpoint + padding.ceil();
        let edge2_line_pos = edge2 - line_midpoint - padding.floor();

        let line1_p1 = self.split_axis.pack_point(edge1_line_pos, 0.);
        let line1_p2 = self.split_axis.pack_point(edge1_line_pos, cross_length);

        let line2_p1 = self.split_axis.pack_point(edge2_line_pos, 0.);
        let line2_p2 = self.split_axis.pack_point(edge2_line_pos, cross_length);

        let (line1, line2) = (Line::new(line1_p1, line1_p2), Line::new(line2_p1, line2_p2));

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
    /// Replaces the first child widget with a new one.
    pub fn set_child1(this: &mut WidgetMut<'_, Self>, child: NewWidget<ChildA>) {
        this.ctx
            .remove_child(std::mem::replace(&mut this.widget.child1, child.to_pod()));
    }

    /// Replaces the second child widget with a new one.
    pub fn set_child2(this: &mut WidgetMut<'_, Self>, child: NewWidget<ChildB>) {
        this.ctx
            .remove_child(std::mem::replace(&mut this.widget.child2, child.to_pod()));
    }

    /// Returns a mutable reference to the first child widget.
    pub fn child1_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, ChildA> {
        this.ctx.get_mut(&mut this.widget.child1)
    }

    /// Returns a mutable reference to the second child widget.
    pub fn child2_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, ChildB> {
        this.ctx.get_mut(&mut this.widget.child2)
    }

    /// Sets the split axis.
    pub fn set_split_axis(this: &mut WidgetMut<'_, Self>, split_axis: Axis) {
        this.widget.split_axis = split_axis;
        this.ctx.request_layout();
    }

    /// Sets the split point as a fraction of the split axis.
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

    /// Sets the minimum lengths for both sides of the split axis.
    ///
    /// The value will be rounded up to the nearest integer.
    pub fn set_min_lengths(this: &mut WidgetMut<'_, Self>, first: Length, second: Length) {
        this.widget.min_lengths = (ceil_length(first), ceil_length(second));
        this.ctx.request_layout();
    }

    /// Sets the thickness of the splitter bar.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar thickness is `6.0`.
    pub fn set_bar_thickness(this: &mut WidgetMut<'_, Self>, bar_thickness: Length) {
        this.widget.bar_thickness = ceil_length(bar_thickness);
        this.ctx.request_layout();
    }

    /// Sets the minimum thickness of the splitter bar area.
    ///
    /// The minimum splitter bar area defines the minimum thickness of the area
    /// where pointer hit detection is done for the splitter bar.
    /// The final hit detection area thickness is either this minimum
    /// or the splitter bar thickness, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area thickness is `6.0`.
    pub fn set_min_bar_area(this: &mut WidgetMut<'_, Self>, min_bar_area: Length) {
        this.widget.min_bar_area = ceil_length(min_bar_area);
        this.ctx.request_layout();
    }

    /// Sets whether the split point can be changed by dragging.
    pub fn set_draggable(this: &mut WidgetMut<'_, Self>, draggable: bool) {
        this.widget.draggable = draggable;
        // Bar mutability impacts appearance, but not accessibility node
        // TODO - This might change in a future implementation
        this.ctx.request_paint_only();
    }

    /// Sets whether the splitter bar is drawn as a solid rectangle.
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
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        if self.draggable {
            match event {
                PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                    let pos = ctx
                        .local_position(state.position)
                        .get_coord(self.split_axis);
                    let length = ctx.size().get_coord(self.split_axis);
                    if self.bar_hit_test(length, pos, scale) {
                        ctx.set_handled();
                        ctx.capture_pointer();
                        // Save the delta between the click position and the split point
                        self.click_offset = pos - self.bar_position(length, scale);
                    }
                }
                PointerEvent::Move(PointerUpdate { current, .. }) => {
                    if ctx.is_active() {
                        let pos = ctx
                            .local_position(current.position)
                            .get_coord(self.split_axis);
                        let length = ctx.size().get_coord(self.split_axis);
                        // If widget has pointer capture, assume always it's hovered
                        let effective_pos = pos - self.click_offset;
                        self.update_split_point(length, effective_pos, scale);
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

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        if let LenReq::FitContent(space) = len_req {
            // We always want to use up all offered space
            if axis == self.split_axis {
                return space.max(self.bar_area(scale));
            }
            return space;
        }
        // Both children can share the same auto length, because it'll be either Min or MaxContent.
        let auto_length = len_req.into();

        let cross = axis.cross();
        let (child1_cross_space, child2_cross_space) = cross_length
            .map(|cross_length| {
                // We need to split the cross length if it's our split axis
                if cross == self.split_axis {
                    let cross_space = (cross_length - self.bar_area(scale)).max(0.);
                    let split_point = self.calc_effective_split_point(cross_space, scale);
                    let child1_cross_space = (cross_space * split_point).floor();
                    (child1_cross_space, cross_space - child1_cross_space)
                } else {
                    (cross_length, cross_length)
                }
            })
            .unzip();
        let child1_context_size = LayoutSize::maybe(cross, child1_cross_space);
        let child2_context_size = LayoutSize::maybe(cross, child2_cross_space);

        let child1_length = ctx.compute_length(
            &mut self.child1,
            auto_length,
            child1_context_size,
            axis,
            child1_cross_space,
        );
        let child2_length = ctx.compute_length(
            &mut self.child2,
            auto_length,
            child2_context_size,
            axis,
            child2_cross_space,
        );

        if axis == self.split_axis {
            child1_length + child2_length + self.bar_area(scale)
        } else {
            child1_length.max(child2_length)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let bar_area = self.bar_area(scale);
        let split_space = (size.get_coord(self.split_axis) - bar_area).max(0.);
        let cross_space = size.get_coord(self.split_axis.cross());

        // Update our effective split point to respect our size
        self.split_point_effective = self.calc_effective_split_point(split_space, scale);

        let child1_split_space = (split_space * self.split_point_effective).floor().max(0.);
        let child2_split_space = (split_space - child1_split_space).max(0.);

        let child1_size = self.split_axis.pack_size(child1_split_space, cross_space);
        let child2_size = self.split_axis.pack_size(child2_split_space, cross_space);

        ctx.run_layout(&mut self.child1, child1_size);
        ctx.run_layout(&mut self.child2, child2_size);

        // Top-left align both children.
        let child1_origin = Point::ORIGIN;
        let child2_origin = self
            .split_axis
            .pack_point(child1_split_space + bar_area, 0.);
        ctx.place_child(&mut self.child1, child1_origin);
        ctx.place_child(&mut self.child2, child2_origin);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // TODO - Paint differently if the bar is draggable and hovered.
        if self.solid {
            self.paint_solid_bar(ctx, scene, scale);
        } else {
            self.paint_stroked_bar(ctx, scene, scale);
        }
        // TODO: Child painting should probably be clipped, in such a way that
        //       one child won't overflow across the split bar onto the other child.
        //       Although that will only happen if we are sized below our MinContent.
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let length = ctx.size().get_coord(self.split_axis);
        let local_pos = (pos - ctx.window_origin().to_vec2()).get_coord(self.split_axis);
        let is_bar_hovered = self.bar_hit_test(length, local_pos, scale);

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
    use crate::theme::test_property_set;
    use crate::widgets::Label;

    #[test]
    fn columns() {
        #[rustfmt::skip]
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        ).split_axis(Axis::Horizontal).draggable(false).with_auto_id();

        let window_size = Size::new(150.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

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
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

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
            .min_lengths(40.px(), 10.px())
            .bar_thickness(12.px())
            .draggable(true)
            .solid_bar(true)
            .with_auto_id();

            let mut harness =
                TestHarness::create_with_size(test_property_set(), widget, Size::new(100.0, 100.0));

            harness.render()
        };

        let image_2 = {
            let widget = Split::new(
                Label::new("Hello").with_auto_id(),
                Label::new("World").with_auto_id(),
            )
            .with_auto_id();

            let mut harness =
                TestHarness::create_with_size(test_property_set(), widget, Size::new(100.0, 100.0));

            harness.edit_root_widget(|mut splitter| {
                Split::set_split_point(&mut splitter, 0.3);
                Split::set_min_lengths(&mut splitter, 40.px(), 10.px());
                Split::set_bar_thickness(&mut splitter, 12.px());
                Split::set_draggable(&mut splitter, true);
                Split::set_bar_solid(&mut splitter, true);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
