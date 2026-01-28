// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{ActionData, Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, CursorIcon, EventCtx, FromDynWidget, LayoutCtx,
    MeasureCtx, NewWidget, NoAction, PaintCtx, PointerButtonEvent, PointerEvent, PointerUpdate,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Line, Point, Rect, Size};
use crate::layout::{AsUnit, LayoutSize, LenReq, Length};
use crate::peniko::Color;
use crate::theme;
use crate::util::{fill_color, stroke};

/// The split point, specifying how the available space is divided between the two children.
///
/// This always applies to the *available space*, which is the widget's size along the split axis
/// minus the splitter bar thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitPoint {
    /// Split by a fraction of the available space.
    ///
    /// `0.0` means the first child gets no space, `1.0` means the second child gets no space.
    /// Values outside `0.0..=1.0` are clamped when set.
    Fraction(f64),
    /// Split by an absolute distance from the start.
    FromStart(Length),
    /// Split by an absolute distance from the end.
    FromEnd(Length),
}

/// A container containing two other widgets, splitting the area either horizontally or vertically.
///
#[doc = concat!(
    "![Split panel with two labels](",
    include_doc_path!("screenshots/split_columns.png"),
    ")",
)]
pub struct Split<ChildA, ChildB>
where
    ChildA: Widget + ?Sized,
    ChildB: Widget + ?Sized,
{
    split_axis: Axis,
    split_point_chosen: SplitPoint,
    split_point_effective: f64,
    min_lengths: (Length, Length),
    bar_thickness: Length,
    min_bar_area: Length,
    solid: bool,
    draggable: bool,
    /// Offset from the bar center to the actual position where the bar was clicked.
    /// This is used to ensure a click without moving is a no-op,
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
            split_point_chosen: SplitPoint::Fraction(0.5),
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
    /// The value is clamped to `0.0..=1.0`.
    ///
    /// The default split point is `0.5`.
    pub fn split_fraction(mut self, split_point: f64) -> Self {
        self.split_point_chosen = SplitPoint::Fraction(split_point.clamp(0.0, 1.0));
        self
    }

    /// Builder-style method to set the split point.
    pub fn split_point(mut self, split_point: SplitPoint) -> Self {
        self.split_point_chosen = match split_point {
            SplitPoint::Fraction(frac) => SplitPoint::Fraction(frac.clamp(0.0, 1.0)),
            other => other,
        };
        self
    }

    /// Builder-style method to set the split point as an absolute distance from the start.
    ///
    /// This is the size of the first child along the split axis.
    /// This can be useful when one side should have a stable pixel size, even when the split
    /// container is resized.
    pub fn split_point_from_start(mut self, split_point: Length) -> Self {
        self.split_point_chosen = SplitPoint::FromStart(split_point);
        self
    }

    /// Builder-style method to set the split point as an absolute distance from the end.
    ///
    /// This is the size of the second child along the split axis.
    /// This can be useful when one side should have a stable pixel size, even when the split
    /// container is resized.
    pub fn split_point_from_end(mut self, split_point: Length) -> Self {
        self.split_point_chosen = SplitPoint::FromEnd(split_point);
        self
    }

    /// Builder-style method to set the minimum length for both sides of the split axis.
    pub fn min_lengths(mut self, first: Length, second: Length) -> Self {
        self.min_lengths = (first, second);
        self
    }

    /// Builder-style method to set the thickness of the splitter bar.
    ///
    /// The default splitter bar thickness is `6.0`.
    pub fn bar_thickness(mut self, bar_thickness: Length) -> Self {
        self.bar_thickness = bar_thickness;
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
    /// The default minimum splitter bar area thickness is `6.0`.
    pub fn min_bar_area(mut self, min_bar_area: Length) -> Self {
        self.min_bar_area = min_bar_area;
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

// --- MARK: METHODS
impl<ChildA: Widget + ?Sized, ChildB: Widget + ?Sized> Split<ChildA, ChildB> {
    /// Returns the thickness of the splitter bar area.
    #[inline]
    fn bar_area(&self, scale: f64) -> f64 {
        self.bar_thickness.max(self.min_bar_area).dp(scale)
    }

    /// Returns the splitter bar center point.
    fn bar_center(&self, length: f64, scale: f64) -> f64 {
        let (edge1, edge2) = self.bar_edges(length, scale);
        (edge1 + edge2) * 0.5
    }

    /// Returns the location of the edges of the splitter bar,
    /// given the specified total length.
    fn bar_edges(&self, length: f64, scale: f64) -> (f64, f64) {
        let bar_thickness = self.bar_thickness.dp(scale);
        let reduced_length = length - bar_thickness;
        let edge = reduced_length * self.split_point_effective;
        (edge, edge + bar_thickness)
    }

    /// Returns the location of the edges of the splitter bar area,
    /// given the specified total length.
    fn bar_area_edges(&self, length: f64, scale: f64) -> (f64, f64) {
        let (edge1, edge2) = self.bar_edges(length, scale);
        let (space1, space2) = (edge1.max(0.), (length - edge2).max(0.));
        let padding = self.bar_area(scale) - self.bar_thickness.dp(scale);

        // Half the padding to the first edge
        let pad1 = (0.5 * padding).min(space1);
        // Remainder to the second edge
        let pad2 = (padding - pad1).min(space2);
        // First edge gets more, in case space2 was low but space1 is high
        let pad1 = (padding - pad2).min(space1);

        (edge1 - pad1, edge2 + pad2)
    }

    /// Returns `true` if the provided position is on the splitter bar area.
    fn bar_area_hit_test(&self, length: f64, pos: f64, scale: f64) -> bool {
        let (edge1, edge2) = self.bar_area_edges(length, scale);
        pos >= edge1 && pos <= edge2
    }

    /// Returns the minimum and maximum split coordinate of the provided length.
    fn split_side_limits(&self, length: f64, scale: f64) -> (f64, f64) {
        let (min_child1, min_child2) = self.min_lengths;
        let mut min_limit = min_child1.dp(scale);
        let mut max_limit = (length - min_child2.dp(scale)).max(0.0);

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
            let child1_len = match self.split_point_chosen {
                SplitPoint::Fraction(frac) => length * frac,
                SplitPoint::FromStart(len) => len.dp(scale),
                SplitPoint::FromEnd(len) => length - len.dp(scale),
            };
            (child1_len / length).clamp(min_limit / length, max_limit / length)
        }
    }

    fn set_chosen_from_child1_len(&mut self, length: f64, child1_len: f64, scale: f64) {
        let (min_limit, max_limit) = self.split_side_limits(length, scale);
        let child1_len = child1_len.clamp(min_limit, max_limit);

        match self.split_point_chosen {
            SplitPoint::Fraction(_) => {
                self.split_point_chosen = SplitPoint::Fraction(if length <= f64::EPSILON {
                    0.5
                } else {
                    child1_len / length
                });
            }
            SplitPoint::FromStart(_) => {
                let logical = child1_len / scale;
                self.split_point_chosen = SplitPoint::FromStart(Length::px(logical));
            }
            SplitPoint::FromEnd(_) => {
                let child2_len = (length - child1_len).max(0.0);
                let logical = child2_len / scale;
                self.split_point_chosen = SplitPoint::FromEnd(Length::px(logical));
            }
        }
    }

    fn update_split_point_from_bar_center(
        &mut self,
        total_length: f64,
        bar_center: f64,
        scale: f64,
    ) {
        let bar_thickness = self.bar_thickness.dp(scale);
        let split_space = (total_length - bar_thickness).max(0.0);
        let child1_len = bar_center - bar_thickness * 0.5;
        self.set_chosen_from_child1_len(split_space, child1_len, scale);
    }

    /// Returns the color of the splitter bar.
    fn bar_color(&self, ctx: &PaintCtx<'_>) -> Color {
        if !self.draggable || ctx.is_disabled() {
            return theme::ZYNC_700;
        }
        if ctx.is_active() || ctx.is_hovered() || ctx.is_focus_target() {
            theme::ZYNC_600
        } else {
            theme::ZYNC_500
        }
    }

    fn paint_solid_bar(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        scene: &mut Scene,
        scale: f64,
        color: Color,
    ) {
        let size = ctx.size();
        let length = size.get_coord(self.split_axis);
        let cross_length = size.get_coord(self.split_axis.cross());
        let (edge1, edge2) = self.bar_edges(length, scale);

        let p1 = self.split_axis.pack_point(edge1, 0.);
        let p2 = self.split_axis.pack_point(edge2, cross_length);
        let rect = Rect::from_points(p1, p2);

        fill_color(scene, &rect, color);
    }

    fn paint_stroked_bar(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        scene: &mut Scene,
        scale: f64,
        color: Color,
    ) {
        let size = ctx.size();
        let length = size.get_coord(self.split_axis);
        let cross_length = size.get_coord(self.split_axis.cross());
        // Set the line width to a third of the splitter bar thickness,
        // because we'll paint two equal lines at the edges.
        let line_width = self.bar_thickness.dp(scale) / 3.0;
        let line_midpoint = line_width / 2.0;
        let (edge1, edge2) = self.bar_edges(length, scale);

        let edge1_line_pos = edge1 + line_midpoint;
        let edge2_line_pos = edge2 - line_midpoint;

        let line1_p1 = self.split_axis.pack_point(edge1_line_pos, 0.);
        let line1_p2 = self.split_axis.pack_point(edge1_line_pos, cross_length);

        let line2_p1 = self.split_axis.pack_point(edge2_line_pos, 0.);
        let line2_p2 = self.split_axis.pack_point(edge2_line_pos, cross_length);

        let (line1, line2) = (Line::new(line1_p1, line1_p2), Line::new(line2_p1, line2_p2));

        stroke(scene, &line1, color, line_width);
        stroke(scene, &line2, color, line_width);
    }
}

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
    pub fn set_split_point(this: &mut WidgetMut<'_, Self>, split_point: SplitPoint) {
        this.widget.split_point_chosen = match split_point {
            SplitPoint::Fraction(frac) => SplitPoint::Fraction(frac.clamp(0.0, 1.0)),
            other => other,
        };
        this.ctx.request_layout();
    }

    /// Sets the minimum lengths for both sides of the split axis.
    pub fn set_min_lengths(this: &mut WidgetMut<'_, Self>, first: Length, second: Length) {
        this.widget.min_lengths = (first, second);
        this.ctx.request_layout();
    }

    /// Sets the thickness of the splitter bar.
    ///
    /// The default splitter bar thickness is `6.0`.
    pub fn set_bar_thickness(this: &mut WidgetMut<'_, Self>, bar_thickness: Length) {
        this.widget.bar_thickness = bar_thickness;
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
    /// The default minimum splitter bar area thickness is `6.0`.
    pub fn set_min_bar_area(this: &mut WidgetMut<'_, Self>, min_bar_area: Length) {
        this.widget.min_bar_area = min_bar_area;
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

    fn accepts_focus(&self) -> bool {
        true
    }

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
                    if self.bar_area_hit_test(length, pos, scale) {
                        ctx.set_handled();
                        ctx.capture_pointer();
                        ctx.request_focus();
                        // Save the delta between the click position and the bar center.
                        self.click_offset = pos - self.bar_center(length, scale);
                    }
                }
                PointerEvent::Move(PointerUpdate { current, .. }) => {
                    if ctx.is_active() {
                        let pos = ctx
                            .local_position(current.position)
                            .get_coord(self.split_axis);
                        let length = ctx.size().get_coord(self.split_axis);
                        // If widget has pointer capture, assume always it's hovered
                        let effective_center = pos - self.click_offset;
                        self.update_split_point_from_bar_center(length, effective_center, scale);
                        ctx.request_layout();
                    }
                }
                PointerEvent::Up(..) | PointerEvent::Cancel(..) => {
                    self.click_offset = 0.0;
                }
                _ => {}
            }
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if ctx.is_disabled() || !ctx.is_focus_target() || !self.draggable {
            return;
        }

        let TextEvent::Keyboard(key_event) = event else {
            return;
        };
        if !key_event.state.is_down() {
            return;
        }

        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let total_length = ctx.size().get_coord(self.split_axis);
        let bar_thickness = self.bar_thickness.dp(scale);
        let split_space = (total_length - bar_thickness).max(0.0);
        if split_space <= f64::EPSILON {
            return;
        }

        let step = (split_space / 100.0).max(1.0);
        let big_step = step * 10.0;
        let delta = if key_event.modifiers.shift() {
            big_step
        } else {
            step
        };

        let mut child1_len = split_space * self.split_point_effective;
        match key_event.key {
            Key::Named(NamedKey::ArrowLeft) if self.split_axis == Axis::Horizontal => {
                child1_len -= delta;
            }
            Key::Named(NamedKey::ArrowRight) if self.split_axis == Axis::Horizontal => {
                child1_len += delta;
            }
            Key::Named(NamedKey::ArrowUp) if self.split_axis == Axis::Vertical => {
                child1_len -= delta;
            }
            Key::Named(NamedKey::ArrowDown) if self.split_axis == Axis::Vertical => {
                child1_len += delta;
            }
            Key::Named(NamedKey::Home) => {
                child1_len = self.split_side_limits(split_space, scale).0;
            }
            Key::Named(NamedKey::End) => {
                child1_len = self.split_side_limits(split_space, scale).1;
            }
            _ => return,
        }

        self.set_chosen_from_child1_len(split_space, child1_len, scale);
        ctx.request_layout();
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.is_disabled() || !self.draggable {
            return;
        }

        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let total_length = ctx.size().get_coord(self.split_axis);
        let bar_thickness = self.bar_thickness.dp(scale);
        let split_space = (total_length - bar_thickness).max(0.0);
        if split_space <= f64::EPSILON {
            return;
        }

        let step = (split_space / 100.0).max(1.0);
        let mut child1_len = split_space * self.split_point_effective;

        match event.action {
            accesskit::Action::Increment => child1_len += step,
            accesskit::Action::Decrement => child1_len -= step,
            accesskit::Action::SetValue => match &event.data {
                Some(ActionData::NumericValue(value)) => child1_len = *value,
                Some(ActionData::Value(value)) => {
                    if let Ok(value) = value.parse() {
                        child1_len = value;
                    }
                }
                _ => return,
            },
            _ => return,
        }

        self.set_chosen_from_child1_len(split_space, child1_len, scale);
        ctx.request_layout();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child1);
        ctx.register_child(&mut self.child2);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FocusChanged(_)
            | Update::HoveredChanged(_)
            | Update::ActiveChanged(_)
            | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
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

        let bar_thickness = self.bar_thickness.dp(scale);

        if let LenReq::FitContent(space) = len_req {
            // We always want to use up all offered space
            if axis == self.split_axis {
                // Don't go below the bar thickness, which we always want to paint.
                return space.max(bar_thickness);
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
                    let cross_space = (cross_length - bar_thickness).max(0.);
                    let split_point = self.calc_effective_split_point(cross_space, scale);
                    let child1_cross_space = cross_space * split_point;
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
            child1_length + child2_length + bar_thickness
        } else {
            child1_length.max(child2_length)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let bar_thickness = self.bar_thickness.dp(scale);
        let split_space = (size.get_coord(self.split_axis) - bar_thickness).max(0.);
        let cross_space = size.get_coord(self.split_axis.cross());

        // Update our effective split point to respect our size
        self.split_point_effective = self.calc_effective_split_point(split_space, scale);

        let child1_split_space = (split_space * self.split_point_effective).max(0.);
        let child2_split_space = (split_space - child1_split_space).max(0.);

        let child1_size = self.split_axis.pack_size(child1_split_space, cross_space);
        let child2_size = self.split_axis.pack_size(child2_split_space, cross_space);

        ctx.run_layout(&mut self.child1, child1_size);
        ctx.run_layout(&mut self.child2, child2_size);

        // Top-left align both children.
        let child1_origin = Point::ORIGIN;
        let child2_origin = self
            .split_axis
            .pack_point(child1_split_space + bar_thickness, 0.);
        ctx.place_child(&mut self.child1, child1_origin);
        ctx.place_child(&mut self.child2, child2_origin);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // TODO - Paint differently if the bar is draggable and hovered.
        let bar_color = self.bar_color(ctx);
        if self.solid {
            self.paint_solid_bar(ctx, scene, scale, bar_color);
        } else {
            self.paint_stroked_bar(ctx, scene, scale, bar_color);
        }

        if ctx.is_focus_target() && self.draggable && !ctx.is_disabled() {
            let size = ctx.size();
            let length = size.get_coord(self.split_axis);
            let cross_length = size.get_coord(self.split_axis.cross());
            let (edge1, edge2) = self.bar_edges(length, scale);

            let p1 = self.split_axis.pack_point(edge1, 0.);
            let p2 = self.split_axis.pack_point(edge2, cross_length);
            let rect = Rect::from_points(p1, p2).inset(2.0);
            let focus_color =
                theme::FOCUS_COLOR.with_alpha(if ctx.is_active() { 1.0 } else { 0.5 });
            stroke(scene, &rect, focus_color, 1.0);
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
        let is_bar_area_hovered = self.bar_area_hit_test(length, local_pos, scale);

        if self.draggable && (ctx.is_active() || is_bar_area_hovered) {
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
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let total_length = ctx.size().get_coord(self.split_axis);
        let bar_thickness = self.bar_thickness.dp(scale);
        let split_space = (total_length - bar_thickness).max(0.0);
        let (min_limit, max_limit) = self.split_side_limits(split_space, scale);
        let child1_len = split_space * self.split_point_effective;

        node.set_orientation(match self.split_axis {
            Axis::Horizontal => accesskit::Orientation::Horizontal,
            Axis::Vertical => accesskit::Orientation::Vertical,
        });
        node.set_value(child1_len.to_string());
        node.set_numeric_value(child1_len);
        node.set_min_numeric_value(min_limit);
        node.set_max_numeric_value(max_limit);
        node.set_numeric_value_step((split_space / 100.0).max(1.0));

        if self.draggable && !ctx.is_disabled() {
            node.add_action(accesskit::Action::SetValue);
            node.add_action(accesskit::Action::Increment);
            node.add_action(accesskit::Action::Decrement);
        }
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
    use crate::core::{PointerButton, TextEvent, WindowEvent};
    use crate::dpi::PhysicalSize;
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

    #[test]
    fn edit_splitter() {
        let image_1 = {
            let widget = Split::new(
                Label::new("Hello").with_auto_id(),
                Label::new("World").with_auto_id(),
            )
            .split_fraction(0.3)
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
                Split::set_split_point(&mut splitter, SplitPoint::Fraction(0.3));
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

    #[test]
    fn drag_moves_split_point() {
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        )
        .with_auto_id();

        let window_size = Size::new(150.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        let child1_initial_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };

        // Initial bar center with default settings:
        // split_space = 150 - 6 = 144, child1 = 72, bar center = 72 + 3 = 75.
        harness.mouse_move(Point::new(75.0, 10.0));
        harness.mouse_button_press(PointerButton::Primary);
        harness.mouse_move(Point::new(105.0, 10.0));
        harness.mouse_button_release(PointerButton::Primary);

        let (child1_width, child2_width) = {
            let root = harness.root_widget();
            let children = root.children();
            (
                children[0].ctx().size().width,
                children[1].ctx().size().width,
            )
        };

        assert!(child1_width > child1_initial_width);
        assert!((child1_width - 102.0).abs() < 0.01);
        assert!((child2_width - 42.0).abs() < 0.01);
    }

    #[test]
    fn keyboard_moves_split_point() {
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        )
        .with_auto_id();

        let window_size = Size::new(150.0, 100.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        let root_id = harness.root_id();
        harness.focus_on(Some(root_id));

        let child1_initial_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };

        harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowRight)));

        let child1_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };

        assert!(child1_width > child1_initial_width);
    }

    #[test]
    fn from_start_keeps_pixel_size_on_resize() {
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        )
        .split_point(SplitPoint::FromStart(50.px()))
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 100.0));

        let child1_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };
        assert!((child1_width - 50.0).abs() < 0.01);

        harness.process_window_event(WindowEvent::Resize(PhysicalSize::new(300, 100)));
        let child1_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };
        assert!((child1_width - 50.0).abs() < 0.01);
    }

    #[test]
    fn from_end_keeps_pixel_size_on_resize() {
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        )
        .split_point(SplitPoint::FromEnd(50.px()))
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 100.0));

        let child2_width = {
            let root = harness.root_widget();
            root.children()[1].ctx().size().width
        };
        assert!((child2_width - 50.0).abs() < 0.01);

        harness.process_window_event(WindowEvent::Resize(PhysicalSize::new(300, 100)));
        let child2_width = {
            let root = harness.root_widget();
            root.children()[1].ctx().size().width
        };
        assert!((child2_width - 50.0).abs() < 0.01);
    }

    #[test]
    fn fraction_clamps_when_set() {
        let widget = Split::new(
            Label::new("Hello").with_auto_id(),
            Label::new("World").with_auto_id(),
        )
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(150.0, 100.0));

        harness.edit_root_widget(|mut split| {
            Split::set_split_point(&mut split, SplitPoint::Fraction(2.0));
        });
        let child1_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };
        assert!((child1_width - 144.0).abs() < 0.01);

        harness.edit_root_widget(|mut split| {
            Split::set_split_point(&mut split, SplitPoint::Fraction(-1.0));
        });
        let child1_width = {
            let root = harness.root_widget();
            root.children()[0].ctx().size().width
        };
        assert!((child1_width - 0.0).abs() < 0.01);
    }
}
