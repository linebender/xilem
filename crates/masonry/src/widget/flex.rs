// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A widget that arranges its children in a one-dimensional array.

use kurbo::{Affine, Stroke};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::Scene;

use crate::kurbo::common::FloatExt;
use crate::kurbo::Vec2;
use crate::theme::get_debug_color;
use crate::widget::{WidgetMut, WidgetRef};
use crate::{
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, PointerEvent,
    Rect, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

/// A container with either horizontal or vertical layout.
///
/// This widget is the foundation of most layouts, and is highly configurable.
pub struct Flex {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    children: Vec<Child>,
}

crate::declare_widget!(FlexMut, Flex);

/// Optional parameters for an item in a [`Flex`] container (row or column).
///
/// Generally, when you would like to add a flexible child to a container,
/// you can simply call [`with_flex_child`](Flex::with_flex_child), passing the
/// child and the desired flex factor as a `f64`, which has an impl of
/// `Into<FlexParams>`.
// FIXME - "with_flex_child or [`add_flex_child`](FlexMut::add_flex_child)"
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FlexParams {
    flex: f64,
    alignment: Option<CrossAxisAlignment>,
}

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    /// The x axis
    Horizontal,
    /// The y axis
    Vertical,
}

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisAlignment {
    /// Top or leading.
    Start,
    /// Widgets are centered in the container.
    Center,
    /// Bottom or trailing.
    End,
    /// Align on the baseline.
    Baseline,
    /// Fill the available space.
    Fill,
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
    /// Top or leading.
    Start,
    /// Children are centered, without padding.
    Center,
    /// Bottom or trailing.
    End,
    /// Extra space is divided evenly between each child.
    SpaceBetween,
    /// Extra space is divided evenly between each child, as well as at the ends.
    SpaceEvenly,
    /// Space between each child, with less at the start and end.
    SpaceAround,
}

// --- Flex impl ---

impl Flex {
    /// Create a new Flex oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Flex {
            direction: axis,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,
        }
    }

    /// Create a new horizontal stack.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    ///
    pub fn row() -> Self {
        Self::for_axis(Axis::Horizontal)
    }

    /// Create a new vertical stack.
    ///
    /// The child widgets are laid out vertically, from top to bottom.
    pub fn column() -> Self {
        Self::for_axis(Axis::Vertical)
    }

    /// Builder-style method for specifying the childrens' [`CrossAxisAlignment`].
    ///
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    /// Builder-style method for specifying the childrens' [`MainAxisAlignment`].
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    /// Builder-style method for setting whether the container must expand
    /// to fill the available space on its main axis.
    pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
        self.fill_major_axis = fill;
        self
    }

    /// Builder-style variant of `add_child`.
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(mut self, child: impl Widget) -> Self {
        let child = Child::Fixed {
            widget: WidgetPod::new(Box::new(child)),
            alignment: None,
        };
        self.children.push(child);
        self
    }

    /// Builder-style variant of `add_child`, that takes the id that the child will have.
    ///
    /// Useful for unit tests.
    pub fn with_child_id(mut self, child: impl Widget, id: WidgetId) -> Self {
        let child = Child::Fixed {
            widget: WidgetPod::new_with_id(Box::new(child), id),
            alignment: None,
        };
        self.children.push(child);
        self
    }

    /// Builder-style method to add a flexible child to the container.
    pub fn with_flex_child(mut self, child: impl Widget, params: impl Into<FlexParams>) -> Self {
        // TODO - dedup?
        let params = params.into();
        let child = if params.flex > 0.0 {
            Child::Flex {
                widget: WidgetPod::new(Box::new(child)),
                alignment: params.alignment,
                flex: params.flex,
            }
        } else {
            // TODO
            tracing::warn!("Flex value should be > 0.0. To add a non-flex child use the add_child or with_child methods.\nSee the docs for masonry::widget::Flex for more information");
            Child::Fixed {
                widget: WidgetPod::new(Box::new(child)),
                alignment: None,
            }
        };
        self.children.push(child);
        self
    }

    /// Builder-style method to add a spacer widget with a standard size.
    ///
    /// The actual value of this spacer depends on whether this container is
    /// a row or column, as well as theme settings.
    pub fn with_default_spacer(self) -> Self {
        let key = match self.direction {
            Axis::Vertical => crate::theme::WIDGET_PADDING_VERTICAL,
            Axis::Horizontal => crate::theme::WIDGET_PADDING_HORIZONTAL,
        };
        self.with_spacer(key)
    }

    /// Builder-style method for adding a fixed-size spacer to the container.
    ///
    /// If you are laying out standard controls in this container, you should
    /// generally prefer to use [`add_default_spacer`].
    ///
    /// [`add_default_spacer`]: #method.add_default_spacer
    pub fn with_spacer(mut self, mut len: f64) -> Self {
        if len < 0.0 {
            tracing::warn!("add_spacer called with negative length: {}", len);
        }
        len = len.clamp(0.0, f64::MAX);

        let new_child = Child::FixedSpacer(len, 0.0);
        self.children.push(new_child);
        self
    }

    /// Builder-style method for adding a `flex` spacer to the container.
    pub fn with_flex_spacer(mut self, flex: f64) -> Self {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        self.children.push(new_child);
        self
    }
}

// --- Mutate live Flex - WidgetMut ---

impl<'a> FlexMut<'a> {
    /// Set the childrens' [`CrossAxisAlignment`].
    ///
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn set_cross_axis_alignment(&mut self, alignment: CrossAxisAlignment) {
        self.widget.cross_alignment = alignment;
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Set the childrens' [`MainAxisAlignment`].
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn set_main_axis_alignment(&mut self, alignment: MainAxisAlignment) {
        self.widget.main_alignment = alignment;
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Set whether the container must expand to fill the available space on
    /// its main axis.
    pub fn set_must_fill_main_axis(&mut self, fill: bool) {
        self.widget.fill_major_axis = fill;
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add a non-flex child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Flex::with_child
    pub fn add_child(&mut self, child: impl Widget) {
        let child = Child::Fixed {
            widget: WidgetPod::new(Box::new(child)),
            alignment: None,
        };
        self.widget.children.push(child);
        // TODO
        self.ctx.widget_state.children_changed = true;
        self.ctx.widget_state.needs_layout = true;
    }

    pub fn add_child_id(&mut self, child: impl Widget, id: WidgetId) {
        let child = Child::Fixed {
            widget: WidgetPod::new_with_id(Box::new(child), id),
            alignment: None,
        };
        self.widget.children.push(child);
        // TODO
        self.ctx.widget_state.children_changed = true;
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add a flexible child widget.
    pub fn add_flex_child(&mut self, child: impl Widget, params: impl Into<FlexParams>) {
        let params = params.into();
        let child = if params.flex > 0.0 {
            Child::Flex {
                widget: WidgetPod::new(Box::new(child)),
                alignment: params.alignment,
                flex: params.flex,
            }
        } else {
            // TODO
            tracing::warn!("Flex value should be > 0.0. To add a non-flex child use the add_child or with_child methods.\nSee the docs for masonry::widget::Flex for more information");
            Child::Fixed {
                widget: WidgetPod::new(Box::new(child)),
                alignment: None,
            }
        };
        self.widget.children.push(child);
        // TODO
        self.ctx.widget_state.children_changed = true;
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add a spacer widget with a standard size.
    ///
    /// The actual value of this spacer depends on whether this container is
    /// a row or column, as well as theme settings.
    pub fn add_default_spacer(&mut self) {
        let key = match self.widget.direction {
            Axis::Vertical => crate::theme::WIDGET_PADDING_VERTICAL,
            Axis::Horizontal => crate::theme::WIDGET_PADDING_HORIZONTAL,
        };
        self.add_spacer(key);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add an empty spacer widget with the given size.
    ///
    /// If you are laying out standard controls in this container, you should
    /// generally prefer to use [`add_default_spacer`].
    ///
    /// [`add_default_spacer`]: Flex::add_default_spacer
    pub fn add_spacer(&mut self, mut len: f64) {
        if len < 0.0 {
            tracing::warn!("add_spacer called with negative length: {}", len);
        }
        len = len.clamp(0.0, f64::MAX);

        let new_child = Child::FixedSpacer(len, 0.0);
        self.widget.children.push(new_child);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add an empty spacer widget with a specific `flex` factor.
    pub fn add_flex_spacer(&mut self, flex: f64) {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        self.widget.children.push(new_child);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add a non-flex child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Flex::with_child
    pub fn insert_child(&mut self, idx: usize, child: impl Widget) {
        let child = Child::Fixed {
            widget: WidgetPod::new(Box::new(child)),
            alignment: None,
        };
        self.widget.children.insert(idx, child);
        // TODO
        self.ctx.widget_state.children_changed = true;
        self.ctx.widget_state.needs_layout = true;
    }

    pub fn insert_flex_child(
        &mut self,
        idx: usize,
        child: impl Widget,
        params: impl Into<FlexParams>,
    ) {
        let params = params.into();
        let child = if params.flex > 0.0 {
            Child::Flex {
                widget: WidgetPod::new(Box::new(child)),
                alignment: params.alignment,
                flex: params.flex,
            }
        } else {
            // TODO
            tracing::warn!("Flex value should be > 0.0. To add a non-flex child use the add_child or with_child methods.\nSee the docs for masonry::widget::Flex for more information");
            Child::Fixed {
                widget: WidgetPod::new(Box::new(child)),
                alignment: None,
            }
        };
        self.widget.children.insert(idx, child);
        // TODO
        self.ctx.widget_state.children_changed = true;
        self.ctx.widget_state.needs_layout = true;
    }

    // TODO - remove
    /// Add a spacer widget with a standard size.
    ///
    /// The actual value of this spacer depends on whether this container is
    /// a row or column, as well as theme settings.
    pub fn insert_default_spacer(&mut self, idx: usize) {
        let key = match self.widget.direction {
            Axis::Vertical => crate::theme::WIDGET_PADDING_VERTICAL,
            Axis::Horizontal => crate::theme::WIDGET_PADDING_HORIZONTAL,
        };
        self.insert_spacer(idx, key);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add an empty spacer widget with the given size.
    ///
    /// If you are laying out standard controls in this container, you should
    /// generally prefer to use [`add_default_spacer`].
    ///
    /// [`add_default_spacer`]: Flex::add_default_spacer
    pub fn insert_spacer(&mut self, idx: usize, mut len: f64) {
        if len < 0.0 {
            tracing::warn!("add_spacer called with negative length: {}", len);
        }
        len = len.clamp(0.0, f64::MAX);

        let new_child = Child::FixedSpacer(len, 0.0);
        self.widget.children.insert(idx, new_child);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    /// Add an empty spacer widget with a specific `flex` factor.
    pub fn insert_flex_spacer(&mut self, idx: usize, flex: f64) {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        self.widget.children.insert(idx, new_child);
        // TODO
        self.ctx.widget_state.needs_layout = true;
    }

    pub fn remove_child(&mut self, idx: usize) {
        self.widget.children.remove(idx);
        self.ctx.widget_state.needs_layout = true;
    }

    // FIXME - Remove Box
    pub fn child_mut(&mut self, idx: usize) -> Option<WidgetMut<'_, Box<dyn Widget>>> {
        let child = match &mut self.widget.children[idx] {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => widget,
            Child::FixedSpacer(..) => return None,
            Child::FlexedSpacer(..) => return None,
        };

        Some(self.ctx.get_mut(child))
    }

    pub fn clear(&mut self) {
        self.widget.children.clear();
        self.ctx.widget_state.needs_layout = true;
    }
}

impl Widget for Flex {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.on_pointer_event(ctx, event);
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.lifecycle(ctx, event);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        // minor-axis values for all children
        let mut minor = self.direction.minor(bc.min());
        // these two are calculated but only used if we're baseline aligned
        let mut max_above_baseline = 0f64;
        let mut max_below_baseline = 0f64;
        let mut any_use_baseline = self.cross_alignment == CrossAxisAlignment::Baseline;

        // Measure non-flex children.
        let mut major_non_flex = 0.0;
        let mut flex_sum = 0.0;
        for child in &mut self.children {
            match child {
                Child::Fixed { widget, alignment } => {
                    any_use_baseline &= *alignment == Some(CrossAxisAlignment::Baseline);

                    let child_bc =
                        self.direction
                            .constraints(&loosened_bc, 0.0, std::f64::INFINITY);
                    let child_size = widget.layout(ctx, &child_bc);
                    let baseline_offset = widget.baseline_offset();

                    if child_size.width.is_infinite() {
                        tracing::warn!("A non-Flex child has an infinite width.");
                    }

                    if child_size.height.is_infinite() {
                        tracing::warn!("A non-Flex child has an infinite height.");
                    }

                    major_non_flex += self.direction.major(child_size).expand();
                    minor = minor.max(self.direction.minor(child_size).expand());
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FixedSpacer(kv, calculated_siz) => {
                    *calculated_siz = *kv;
                    if *calculated_siz < 0.0 {
                        tracing::warn!("Length provided to fixed spacer was less than 0");
                    }
                    *calculated_siz = calculated_siz.max(0.0);
                    major_non_flex += *calculated_siz;
                }
                Child::Flex { flex, .. } | Child::FlexedSpacer(flex, _) => flex_sum += *flex,
            }
        }

        let total_major = self.direction.major(bc.max());
        let remaining = (total_major - major_non_flex).max(0.0);
        let mut remainder: f64 = 0.0;

        let mut major_flex: f64 = 0.0;
        let px_per_flex = remaining / flex_sum;
        // Measure flex children.
        for child in &mut self.children {
            match child {
                Child::Flex { widget, flex, .. } => {
                    let desired_major = (*flex) * px_per_flex + remainder;
                    let actual_major = desired_major.round();
                    remainder = desired_major - actual_major;

                    let child_bc = self.direction.constraints(&loosened_bc, 0.0, actual_major);
                    let child_size = widget.layout(ctx, &child_bc);
                    let baseline_offset = widget.baseline_offset();

                    major_flex += self.direction.major(child_size).expand();
                    minor = minor.max(self.direction.minor(child_size).expand());
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FlexedSpacer(flex, calculated_size) => {
                    let desired_major = (*flex) * px_per_flex + remainder;
                    *calculated_size = desired_major.round();
                    remainder = desired_major - *calculated_size;
                    major_flex += *calculated_size;
                }
                _ => {}
            }
        }

        // figure out if we have extra space on major axis, and if so how to use it
        let extra = if self.fill_major_axis {
            (remaining - major_flex).max(0.0)
        } else {
            // if we are *not* expected to fill our available space this usually
            // means we don't have any extra, unless dictated by our constraints.
            (self.direction.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
        };

        let mut spacing = Spacing::new(self.main_alignment, extra, self.children.len());

        // the actual size needed to tightly fit the children on the minor axis.
        // Unlike the 'minor' var, this ignores the incoming constraints.
        let minor_dim = match self.direction {
            Axis::Horizontal if any_use_baseline => max_below_baseline + max_above_baseline,
            _ => minor,
        };

        let extra_height = minor - minor_dim.min(minor);

        let mut major = spacing.next().unwrap_or(0.);

        for child in &mut self.children {
            match child {
                Child::Fixed { widget, alignment }
                | Child::Flex {
                    widget, alignment, ..
                } => {
                    let child_size = widget.layout_rect().size();
                    let alignment = alignment.unwrap_or(self.cross_alignment);
                    let child_minor_offset = match alignment {
                        // This will ignore baseline alignment if it is overridden on children,
                        // but is not the default for the container. Is this okay?
                        CrossAxisAlignment::Baseline
                            if matches!(self.direction, Axis::Horizontal) =>
                        {
                            let child_baseline = widget.baseline_offset();
                            let child_above_baseline = child_size.height - child_baseline;
                            extra_height + (max_above_baseline - child_above_baseline)
                        }
                        CrossAxisAlignment::Fill => {
                            let fill_size: Size = self
                                .direction
                                .pack(self.direction.major(child_size), minor_dim)
                                .into();
                            let child_bc = BoxConstraints::tight(fill_size);
                            widget.layout(ctx, &child_bc);
                            0.0
                        }
                        _ => {
                            let extra_minor = minor_dim - self.direction.minor(child_size);
                            alignment.align(extra_minor)
                        }
                    };

                    let child_pos: Point = self.direction.pack(major, child_minor_offset).into();
                    ctx.place_child(widget, child_pos);
                    major += self.direction.major(child_size).expand();
                    major += spacing.next().unwrap_or(0.);
                }
                Child::FlexedSpacer(_, calculated_size)
                | Child::FixedSpacer(_, calculated_size) => {
                    major += *calculated_size;
                }
            }
        }

        if flex_sum > 0.0 && total_major.is_infinite() {
            tracing::warn!("A child of Flex is flex, but Flex is unbounded.")
        }

        if flex_sum > 0.0 {
            major = total_major;
        }

        let my_size: Size = self.direction.pack(major, minor_dim).into();

        // if we don't have to fill the main axis, we loosen that axis before constraining
        let my_size = if !self.fill_major_axis {
            let max_major = self.direction.major(bc.max());
            self.direction
                .constraints(bc, 0.0, max_major)
                .constrain(my_size)
        } else {
            bc.constrain(my_size)
        };

        let baseline_offset = match self.direction {
            Axis::Horizontal => max_below_baseline,
            Axis::Vertical => (self.children)
                .last()
                .map(|last| {
                    let child = last.widget();
                    if let Some(widget) = child {
                        let child_bl = widget.baseline_offset();
                        let child_max_y = widget.layout_rect().max_y();
                        let extra_bottom_padding = my_size.height - child_max_y;
                        child_bl + extra_bottom_padding
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0),
        };

        ctx.set_baseline_offset(baseline_offset);
        trace!(
            "Computed layout: size={}, baseline_offset={}",
            my_size,
            baseline_offset
        );
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.paint(ctx, scene);
        }

        // paint the baseline if we're debugging layout
        if ctx.debug_paint && ctx.widget_state.baseline_offset != 0.0 {
            let color = get_debug_color(ctx.widget_id().to_raw());
            let my_baseline = ctx.size().height - ctx.widget_state.baseline_offset;
            let line = crate::kurbo::Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.as_dyn())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Flex")
    }
}

// --- Others impls ---

impl Axis {
    /// Get the axis perpendicular to this one.
    pub fn cross(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    /// Extract from the argument the magnitude along this axis
    pub fn major(self, size: Size) -> f64 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    /// Extract from the argument the magnitude along the perpendicular axis
    pub fn minor(self, size: Size) -> f64 {
        self.cross().major(size)
    }

    /// Extract the extent of the argument in this axis as a pair.
    pub fn major_span(self, rect: Rect) -> (f64, f64) {
        match self {
            Axis::Horizontal => (rect.x0, rect.x1),
            Axis::Vertical => (rect.y0, rect.y1),
        }
    }

    /// Extract the extent of the argument in the minor axis as a pair.
    pub fn minor_span(self, rect: Rect) -> (f64, f64) {
        self.cross().major_span(rect)
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_pos(self, pos: Point) -> f64 {
        match self {
            Axis::Horizontal => pos.x,
            Axis::Vertical => pos.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_vec(self, vec: Vec2) -> f64 {
        match self {
            Axis::Horizontal => vec.x,
            Axis::Vertical => vec.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to the perpendicular axis.
    pub fn minor_pos(self, pos: Point) -> f64 {
        self.cross().major_pos(pos)
    }

    /// Extract the coordinate locating the argument with respect to the perpendicular axis.
    pub fn minor_vec(self, vec: Vec2) -> f64 {
        self.cross().major_vec(vec)
    }

    // TODO - make_pos, make_size, make_rect
    /// Arrange the major and minor measurements with respect to this axis such that it forms
    /// an (x, y) pair.
    pub fn pack(self, major: f64, minor: f64) -> (f64, f64) {
        match self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }

    /// Generate constraints with new values on the major axis.
    pub(crate) fn constraints(
        self,
        bc: &BoxConstraints,
        min_major: f64,
        major: f64,
    ) -> BoxConstraints {
        match self {
            Axis::Horizontal => BoxConstraints::new(
                Size::new(min_major, bc.min().height),
                Size::new(major, bc.max().height),
            ),
            Axis::Vertical => BoxConstraints::new(
                Size::new(bc.min().width, min_major),
                Size::new(bc.max().width, major),
            ),
        }
    }
}

impl FlexParams {
    /// Create custom `FlexParams` with a specific `flex_factor` and an optional
    /// [`CrossAxisAlignment`].
    ///
    /// You likely only need to create these manually if you need to specify
    /// a custom alignment; if you only need to use a custom `flex_factor` you
    /// can pass an `f64` to any of the functions that take `FlexParams`.
    ///
    /// By default, the widget uses the alignment of its parent [`Flex`] container.
    pub fn new(flex: f64, alignment: impl Into<Option<CrossAxisAlignment>>) -> Self {
        if flex <= 0.0 {
            debug_panic!("Flex value should be > 0.0. Flex given was: {}", flex);
        }

        let flex = flex.max(0.0);

        FlexParams {
            flex,
            alignment: alignment.into(),
        }
    }
}

impl CrossAxisAlignment {
    /// Given the difference between the size of the container and the size
    /// of the child (on their minor axis) return the necessary offset for
    /// this alignment.
    fn align(self, val: f64) -> f64 {
        match self {
            CrossAxisAlignment::Start => 0.0,
            // in vertical layout, baseline is equivalent to center
            CrossAxisAlignment::Center | CrossAxisAlignment::Baseline => (val / 2.0).round(),
            CrossAxisAlignment::End => val,
            CrossAxisAlignment::Fill => 0.0,
        }
    }
}

struct Spacing {
    alignment: MainAxisAlignment,
    extra: f64,
    n_children: usize,
    index: usize,
    equal_space: f64,
    remainder: f64,
}

impl Spacing {
    /// Given the provided extra space and children count,
    /// this returns an iterator of `f64` spacing,
    /// where the first element is the spacing before any children
    /// and all subsequent elements are the spacing after children.
    fn new(alignment: MainAxisAlignment, extra: f64, n_children: usize) -> Spacing {
        let extra = if extra.is_finite() { extra } else { 0. };
        let equal_space = if n_children > 0 {
            match alignment {
                MainAxisAlignment::Center => extra / 2.,
                MainAxisAlignment::SpaceBetween => extra / (n_children - 1).max(1) as f64,
                MainAxisAlignment::SpaceEvenly => extra / (n_children + 1) as f64,
                MainAxisAlignment::SpaceAround => extra / (2 * n_children) as f64,
                _ => 0.,
            }
        } else {
            0.
        };
        Spacing {
            alignment,
            extra,
            n_children,
            index: 0,
            equal_space,
            remainder: 0.,
        }
    }

    fn next_space(&mut self) -> f64 {
        let desired_space = self.equal_space + self.remainder;
        let actual_space = desired_space.round();
        self.remainder = desired_space - actual_space;
        actual_space
    }
}

impl Iterator for Spacing {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        if self.index > self.n_children {
            return None;
        }
        let result = {
            if self.n_children == 0 {
                self.extra
            } else {
                #[allow(clippy::match_bool)]
                match self.alignment {
                    MainAxisAlignment::Start => match self.index == self.n_children {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::End => match self.index == 0 {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::Center => match self.index {
                        0 => self.next_space(),
                        i if i == self.n_children => self.next_space(),
                        _ => 0.,
                    },
                    MainAxisAlignment::SpaceBetween => match self.index {
                        0 => 0.,
                        i if i != self.n_children => self.next_space(),
                        _ => match self.n_children {
                            1 => self.next_space(),
                            _ => 0.,
                        },
                    },
                    MainAxisAlignment::SpaceEvenly => self.next_space(),
                    MainAxisAlignment::SpaceAround => {
                        if self.index == 0 || self.index == self.n_children {
                            self.next_space()
                        } else {
                            self.next_space() + self.next_space()
                        }
                    }
                }
            }
        };
        self.index += 1;
        Some(result)
    }
}

impl From<f64> for FlexParams {
    fn from(flex: f64) -> FlexParams {
        FlexParams::new(flex, None)
    }
}

enum Child {
    Fixed {
        widget: WidgetPod<Box<dyn Widget>>,
        alignment: Option<CrossAxisAlignment>,
    },
    Flex {
        widget: WidgetPod<Box<dyn Widget>>,
        alignment: Option<CrossAxisAlignment>,
        flex: f64,
    },
    FixedSpacer(f64, f64),
    FlexedSpacer(f64, f64),
}

impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<Box<dyn Widget>>> {
        match self {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
    fn widget(&self) -> Option<&WidgetPod<Box<dyn Widget>>> {
        match self {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Label;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_main_axis_alignment_spacing() {
        // The following alignment strategy is based on how
        // Chrome 80 handles it with CSS flex.

        let vec = |a, e, n| -> Vec<f64> { Spacing::new(a, e, n).collect() };

        let a = MainAxisAlignment::Start;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 0., 10.]);
        assert_eq!(vec(a, 10., 3), vec![0., 0., 0., 10.]);

        let a = MainAxisAlignment::End;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![10., 0.]);
        assert_eq!(vec(a, 10., 2), vec![10., 0., 0.]);
        assert_eq!(vec(a, 10., 3), vec![10., 0., 0., 0.]);

        let a = MainAxisAlignment::Center;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![5., 0., 5.]);
        assert_eq!(vec(a, 10., 3), vec![5., 0., 0., 5.]);
        assert_eq!(vec(a, 1., 0), vec![1.]);
        assert_eq!(vec(a, 3., 1), vec![2., 1.]);
        assert_eq!(vec(a, 5., 2), vec![3., 0., 2.]);
        assert_eq!(vec(a, 17., 3), vec![9., 0., 0., 8.]);

        let a = MainAxisAlignment::SpaceBetween;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 10., 0.]);
        assert_eq!(vec(a, 10., 3), vec![0., 5., 5., 0.]);
        assert_eq!(vec(a, 33., 5), vec![0., 8., 9., 8., 8., 0.]);
        assert_eq!(vec(a, 34., 5), vec![0., 9., 8., 9., 8., 0.]);
        assert_eq!(vec(a, 35., 5), vec![0., 9., 9., 8., 9., 0.]);
        assert_eq!(vec(a, 36., 5), vec![0., 9., 9., 9., 9., 0.]);
        assert_eq!(vec(a, 37., 5), vec![0., 9., 10., 9., 9., 0.]);
        assert_eq!(vec(a, 38., 5), vec![0., 10., 9., 10., 9., 0.]);
        assert_eq!(vec(a, 39., 5), vec![0., 10., 10., 9., 10., 0.]);

        let a = MainAxisAlignment::SpaceEvenly;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 4., 3.]);
        assert_eq!(vec(a, 10., 3), vec![3., 2., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![6., 5., 6., 5., 6., 5.]);
        assert_eq!(vec(a, 34., 5), vec![6., 5., 6., 6., 5., 6.]);
        assert_eq!(vec(a, 35., 5), vec![6., 6., 5., 6., 6., 6.]);
        assert_eq!(vec(a, 36., 5), vec![6., 6., 6., 6., 6., 6.]);
        assert_eq!(vec(a, 37., 5), vec![6., 6., 7., 6., 6., 6.]);
        assert_eq!(vec(a, 38., 5), vec![6., 7., 6., 6., 7., 6.]);
        assert_eq!(vec(a, 39., 5), vec![7., 6., 7., 6., 7., 6.]);

        let a = MainAxisAlignment::SpaceAround;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 5., 2.]);
        assert_eq!(vec(a, 10., 3), vec![2., 3., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![3., 7., 6., 7., 7., 3.]);
        assert_eq!(vec(a, 34., 5), vec![3., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 35., 5), vec![4., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 36., 5), vec![4., 7., 7., 7., 7., 4.]);
        assert_eq!(vec(a, 37., 5), vec![4., 7., 8., 7., 7., 4.]);
        assert_eq!(vec(a, 38., 5), vec![4., 7., 8., 8., 7., 4.]);
        assert_eq!(vec(a, 39., 5), vec![4., 8., 7., 8., 8., 4.]);
    }

    // TODO - fix this test
    #[test]
    #[should_panic]
    fn test_invalid_flex_params() {
        use float_cmp::approx_eq;
        let params = FlexParams::new(0.0, None);
        approx_eq!(f64, params.flex, 1.0, ulps = 2);

        let params = FlexParams::new(-0.0, None);
        approx_eq!(f64, params.flex, 1.0, ulps = 2);

        let params = FlexParams::new(-1.0, None);
        approx_eq!(f64, params.flex, 1.0, ulps = 2);
    }

    // TODO - Reduce copy-pasting?
    #[test]
    fn flex_row_cross_axis_snapshots() {
        let widget = Flex::row()
            .with_child(Label::new("hello"))
            .with_flex_child(Label::new("world"), 1.0)
            .with_child(Label::new("foo"))
            .with_flex_child(
                Label::new("bar"),
                FlexParams::new(2.0, CrossAxisAlignment::Start),
            );

        let mut harness = TestHarness::create(widget);

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "row_cross_axis_start");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "row_cross_axis_center");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "row_cross_axis_end");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "row_cross_axis_baseline");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "row_cross_axis_fill");
    }

    #[test]
    fn flex_row_main_axis_snapshots() {
        let widget = Flex::row()
            .with_child(Label::new("hello"))
            .with_flex_child(Label::new("world"), 1.0)
            .with_child(Label::new("foo"))
            .with_flex_child(
                Label::new("bar"),
                FlexParams::new(2.0, CrossAxisAlignment::Start),
            );

        let mut harness = TestHarness::create(widget);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "row_main_axis_start");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "row_main_axis_center");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "row_main_axis_end");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "row_main_axis_spaceBetween");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "row_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "row_main_axis_spaceAround");

        // FILL MAIN AXIS
        // TODO - This doesn't seem to do anything?

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_must_fill_main_axis(true);
        });
        assert_render_snapshot!(harness, "row_fill_main_axis");
    }

    #[test]
    fn flex_col_cross_axis_snapshots() {
        let widget = Flex::column()
            .with_child(Label::new("hello"))
            .with_flex_child(Label::new("world"), 1.0)
            .with_child(Label::new("foo"))
            .with_flex_child(
                Label::new("bar"),
                FlexParams::new(2.0, CrossAxisAlignment::Start),
            );

        let mut harness = TestHarness::create(widget);

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "col_cross_axis_start");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "col_cross_axis_center");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "col_cross_axis_end");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "col_cross_axis_baseline");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_cross_axis_alignment(CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "col_cross_axis_fill");
    }

    #[test]
    fn flex_col_main_axis_snapshots() {
        let widget = Flex::column()
            .with_child(Label::new("hello"))
            .with_flex_child(Label::new("world"), 1.0)
            .with_child(Label::new("foo"))
            .with_flex_child(
                Label::new("bar"),
                FlexParams::new(2.0, CrossAxisAlignment::Start),
            );

        let mut harness = TestHarness::create(widget);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "col_main_axis_start");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "col_main_axis_center");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "col_main_axis_end");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "col_main_axis_spaceBetween");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "col_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_main_axis_alignment(MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "col_main_axis_spaceAround");

        // FILL MAIN AXIS
        // TODO - This doesn't seem to do anything?

        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();
            flex.set_must_fill_main_axis(true);
        });
        assert_render_snapshot!(harness, "col_fill_main_axis");
    }

    #[test]
    fn edit_flex_container() {
        let image_1 = {
            let widget = Flex::column()
                .with_child(Label::new("a"))
                .with_child(Label::new("b"))
                .with_child(Label::new("c"))
                .with_child(Label::new("d"));
            // -> abcd

            let mut harness = TestHarness::create(widget);

            harness.edit_root_widget(|mut flex| {
                let mut flex = flex.downcast::<Flex>().unwrap();

                flex.remove_child(1);
                // -> acd
                flex.add_child(Label::new("x"));
                // -> acdx
                flex.add_flex_child(Label::new("y"), 2.0);
                // -> acdxy
                flex.add_default_spacer();
                // -> acdxy_
                flex.add_spacer(5.0);
                // -> acdxy__
                flex.add_flex_spacer(1.0);
                // -> acdxy___
                flex.insert_child(2, Label::new("i"));
                // -> acidxy___
                flex.insert_flex_child(2, Label::new("j"), 2.0);
                // -> acjidxy___
                flex.insert_default_spacer(2);
                // -> ac_jidxy___
                flex.insert_spacer(2, 5.0);
                // -> ac__jidxy___
                flex.insert_flex_spacer(2, 1.0);
            });

            harness.render()
        };

        let image_2 = {
            let widget = Flex::column()
                .with_child(Label::new("a"))
                .with_child(Label::new("c"))
                .with_flex_spacer(1.0)
                .with_spacer(5.0)
                .with_default_spacer()
                .with_flex_child(Label::new("j"), 2.0)
                .with_child(Label::new("i"))
                .with_child(Label::new("d"))
                .with_child(Label::new("x"))
                .with_flex_child(Label::new("y"), 2.0)
                .with_default_spacer()
                .with_spacer(5.0)
                .with_flex_spacer(1.0);

            let mut harness = TestHarness::create(widget);
            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }

    #[test]
    fn get_flex_child() {
        let widget = Flex::column()
            .with_child(Label::new("hello"))
            .with_child(Label::new("world"))
            .with_spacer(1.0);

        let mut harness = TestHarness::create(widget);
        harness.edit_root_widget(|mut flex| {
            let mut flex = flex.downcast::<Flex>().unwrap();

            let mut child = flex.child_mut(1).unwrap();
            assert_eq!(
                child.downcast::<Label>().unwrap().text().to_string(),
                "world"
            );
            std::mem::drop(child);

            assert!(flex.child_mut(2).is_none());
        });

        // TODO - test out-of-bounds access?
    }
}
