// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in a one-dimensional array.

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::debug_panic;
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::common::FloatExt;
use vello::kurbo::{Affine, Line, Point, Size, Stroke};

use crate::core::{
    AccessCtx, Axis, BoxConstraints, LayoutCtx, PaintCtx, PropertiesMut, PropertiesRef,
    RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::util::{fill, stroke};

/// A non-flex container with either horizontal or vertical layout.
pub struct List {
    direction: Axis,
    children: Vec<WidgetPod<dyn Widget>>,
    gap: f64,
}

// --- MARK: IMPL LIST
impl List {
    /// Create a new `Flex` oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Self {
            direction: axis,
            children: Vec::new(),
            gap: 0.,
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

    /// Builder-style variant of [`List::set_gap`].
    pub fn gap(mut self, mut gap: f64) -> Self {
        if !gap.is_finite() || gap < 0.0 {
            debug_panic!("Invalid gap value '{gap}', expected a non-negative finite value.");
            gap = 0.0;
        }
        self.gap = gap;
        self
    }

    /// Builder-style variant of [`List::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(self, child: impl Widget) -> Self {
        self.with_child_pod(WidgetPod::new(child).erased())
    }

    /// Builder-style method for [adding](List::add_child) a type-erased child to this.
    pub fn with_child_pod(mut self, child: WidgetPod<dyn Widget>) -> Self {
        self.children.push(child);
        self
    }

    /// Returns the number of children (widgets and spacers) this flex container has.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this flex container has no children (widgets or spacers).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// --- MARK: WIDGETMUT
impl List {
    /// Set the direction (see [`Axis`]).
    pub fn set_direction(this: &mut WidgetMut<'_, Self>, direction: Axis) {
        this.widget.direction = direction;
        this.ctx.request_layout();
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Equivalent to the css [gap] property.
    /// This gap is also present between spacers.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    ///
    /// ## Panics
    ///
    /// If `gap` is not a non-negative finite value.
    pub fn set_gap(this: &mut WidgetMut<'_, Self>, mut gap: f64) {
        if !gap.is_finite() || gap < 0.0 {
            debug_panic!("Invalid gap value '{gap}', expected a non-negative finite value.");
            gap = 0.0;
        }
        this.widget.gap = gap;
        this.ctx.request_layout();
    }

    /// Add a non-flex child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: List::with_child
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: impl Widget) {
        this.widget.children.push(WidgetPod::new(child).erased());
        this.ctx.children_changed();
    }

    /// Insert a non-flex child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child(this: &mut WidgetMut<'_, Self>, idx: usize, child: impl Widget) {
        Self::insert_child_pod(this, idx, WidgetPod::new(child).erased());
    }

    /// Insert a non-flex child widget wrapped in a [`WidgetPod`] at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child_pod(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: WidgetPod<dyn Widget>,
    ) {
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Remove the child at `idx`.
    ///
    /// This child can be a widget or a spacer.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child);
        this.ctx.request_layout();
    }

    /// Returns a mutable reference to the child widget at `idx`.
    ///
    /// Returns `None` if the child at `idx` is a spacer.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.children[idx];
        this.ctx.get_mut(child)
    }

    /// Remove all children from the container.
    pub fn clear(this: &mut WidgetMut<'_, Self>) {
        if this.widget.children.is_empty() {
            return;
        }

        this.ctx.request_layout();

        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child);
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for List {
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut() {
            ctx.register_child(child);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        // minor-axis values for all children
        let mut minor = self.direction.minor(bc.min());

        let gap = self.gap;

        for child in &mut self.children {
            let child_size = ctx.run_layout(child, &loosened_bc);
            minor = minor.max(self.direction.minor(child_size).expand());
        }

        let mut major = 0.;
        for child in &mut self.children {
            let child_size = ctx.child_size(child);
            let extra_minor = minor - self.direction.minor(child_size);
            let child_minor_offset = (extra_minor / 2.).round();

            let child_pos: Point = self.direction.pack(major, child_minor_offset).into();
            let child_pos = border.place_down(child_pos);
            let child_pos = padding.place_down(child_pos);
            ctx.place_child(child, child_pos);
            major += self.direction.major(child_size).expand();
            major += gap;
        }

        if !self.children.is_empty() {
            // If we have at least one child, the last child added `gap` to `major`, which means that `major` is
            // not the total size of the flex in the major axis, it's instead where the "next widget" will be placed.
            // However, for the rest of this value, we need the total size of the widget in the major axis.
            major -= gap;
        }

        // my_size may be larger than the given constraints.
        // In which case, the Flex widget will either overflow its parent
        // or be clipped (e.g. if its parent is a Portal).
        let my_size: Size = self.direction.pack(major, minor).into();
        let baseline = 0.;

        let (my_size, baseline) = padding.layout_up(my_size, baseline);
        let (my_size, baseline) = border.layout_up(my_size, baseline);
        ctx.set_baseline_offset(baseline);
        my_size
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

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("List", id = id.trace())
    }
}
