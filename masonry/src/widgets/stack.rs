// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in a one-dimensional array.

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Line, Point, Size, Stroke};

use crate::core::{
    AccessCtx, Axis, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::theme::DEFAULT_GAP;
use crate::util::{debug_panic, fill, include_screenshot, stroke};
use crate::widgets::flex::get_spacing;

/// A container with either horizontal or vertical layout.
///
#[doc = include_screenshot!("stack_col_main_axis_spaceAround.png", "Column with multiple labels.")]
pub struct Stack {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    children: Vec<Child>,
    gap: f64,
}

struct Child {
    widget: WidgetPod<dyn Widget>,
    alignment: Option<CrossAxisAlignment>,
}

// --- MARK: IMPL STACK
impl Stack {
    /// Create a new `Stack` oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Self {
            direction: axis,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
            gap: DEFAULT_GAP,
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

    /// Builder-style method for specifying the children's [`CrossAxisAlignment`].
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    /// Builder-style method for specifying the children's [`MainAxisAlignment`].
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    /// Builder-style method for setting a gap along the
    /// major axis between any two elements in logical pixels.
    ///
    /// By default this is [`DEFAULT_GAP`].
    ///
    /// Similar to the css [gap] property.
    ///
    /// # Panics
    ///
    /// If `gap` is not a non-negative finite value.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    pub fn with_gap(mut self, mut gap: f64) -> Self {
        if !gap.is_finite() || gap < 0.0 {
            debug_panic!("Invalid gap value '{gap}', expected a non-negative finite value.");
            gap = 0.0;
        }
        self.gap = gap;
        self
    }

    /// Builder-style variant of [`Stack::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(
        mut self,
        child: NewWidget<impl Widget + ?Sized>,
        alignment: Option<CrossAxisAlignment>,
    ) -> Self {
        let child = Child {
            widget: child.erased().to_pod(),
            alignment,
        };
        self.children.push(child);
        self
    }

    /// Returns the number of child widgets this container has.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this container has no child widget.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// --- MARK: WIDGETMUT
impl Stack {
    /// Set the direction this container grows in (see [`Axis`]).
    pub fn set_direction(this: &mut WidgetMut<'_, Self>, direction: Axis) {
        this.widget.direction = direction;
        this.ctx.request_layout();
    }

    /// Set the children's [`CrossAxisAlignment`].
    pub fn set_cross_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: CrossAxisAlignment) {
        this.widget.cross_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Set the children's [`MainAxisAlignment`].
    pub fn set_main_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: MainAxisAlignment) {
        this.widget.main_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Similar to the css [gap] property.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    ///
    /// # Panics
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

    /// Add a child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Stack::with_child
    pub fn add_child(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        alignment: Option<CrossAxisAlignment>,
    ) {
        let child = Child {
            widget: child.erased().to_pod(),
            alignment,
        };
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Insert a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        alignment: Option<CrossAxisAlignment>,
    ) {
        let child = Child {
            widget: child.erased().to_pod(),
            alignment,
        };
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
        this.ctx.remove_child(child.widget);
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
        this.ctx.get_mut(&mut child.widget)
    }

    /// Updates the alignment for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn update_child_alignment(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        alignment: Option<CrossAxisAlignment>,
    ) {
        let child = &mut this.widget.children[idx];
        child.alignment = alignment;
        this.ctx.request_layout();
    }

    /// Remove all children from the container.
    pub fn clear(this: &mut WidgetMut<'_, Self>) {
        if !this.widget.children.is_empty() {
            this.ctx.request_layout();

            for child in this.widget.children.drain(..) {
                this.ctx.remove_child(child.widget);
            }
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Stack {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(&mut child.widget);
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
        // SETUP
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        let gap_count = self.children.len().saturating_sub(1);

        // ACCUMULATORS
        let mut minor = self.direction.minor(bc.min());
        let mut major = gap_count as f64 * self.gap;
        // Values used if any child has `CrossAxisAlignment::Baseline`.
        let mut max_above_baseline = 0_f64;
        let mut max_below_baseline = 0_f64;

        // MEASURE CHILDREN
        for child in &mut self.children {
            let child_size = ctx.run_layout(&mut child.widget, &loosened_bc);

            let baseline_offset = ctx.child_baseline_offset(&child.widget);

            major += self.direction.major(child_size);
            minor = minor.max(self.direction.minor(child_size));
            max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
            max_below_baseline = max_below_baseline.max(baseline_offset);
        }

        // COMPUTE EXTRA SPACE
        let extra_length = (self.direction.major(bc.min()) - major).max(0.0);
        let (space_before, space_between) =
            get_spacing(self.main_alignment, extra_length, self.children.len());

        // DISTRIBUTE EXTRA SPACE
        let mut major_progress = space_before;
        for child in &mut self.children {
            let child_size = ctx.child_size(&child.widget);
            let alignment = child.alignment.unwrap_or(self.cross_alignment);
            let child_minor_offset = match alignment {
                CrossAxisAlignment::Baseline if self.direction == Axis::Horizontal => {
                    let max_height = max_below_baseline + max_above_baseline;
                    let extra_height = (minor - max_height).max(0.);

                    let child_baseline = ctx.child_baseline_offset(&child.widget);
                    let child_above_baseline = child_size.height - child_baseline;
                    extra_height + (max_above_baseline - child_above_baseline)
                }
                CrossAxisAlignment::Fill => {
                    let fill_size: Size = self
                        .direction
                        .pack(self.direction.major(child_size), minor)
                        .into();
                    let child_bc = BoxConstraints::tight(fill_size);
                    // TODO: This is the second call of layout on the same child,
                    // which can lead to exponential increase in layout calls
                    // when used multiple times in the widget hierarchy.
                    ctx.run_layout(&mut child.widget, &child_bc);
                    0.0
                }
                _ => {
                    let extra_minor = minor - self.direction.minor(child_size);
                    alignment.align(extra_minor)
                }
            };

            let child_pos: Point = self
                .direction
                .pack(major_progress, child_minor_offset)
                .into();
            let child_pos = border.place_down(child_pos);
            let child_pos = padding.place_down(child_pos);
            ctx.place_child(&mut child.widget, child_pos);

            major_progress += self.direction.major(child_size);
            major_progress += space_between;
            major_progress += self.gap;
        }

        let my_size: Size = self.direction.pack(major, minor).into();

        let baseline = match self.direction {
            Axis::Horizontal => max_below_baseline,
            Axis::Vertical => self
                .children
                .last()
                .map(|last| {
                    let widget = &last.widget;
                    let child_bl = ctx.child_baseline_offset(widget);
                    let child_max_y = ctx.child_layout_rect(widget).max_y();
                    let extra_bottom_padding = my_size.height - child_max_y;
                    child_bl + extra_bottom_padding
                })
                .unwrap_or(0.0),
        };

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

    fn children_ids(&self) -> ChildrenIds {
        self.children
            .iter()
            .map(|child| child.widget.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Stack", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::Label;

    // TODO - Reduce copy-pasting?
    #[test]
    fn stack_row_cross_axis_snapshots() {
        let widget = Stack::row()
            .with_child(Label::new("hello").with_auto_id(), None)
            .with_child(Label::new("world").with_auto_id(), None)
            .with_child(
                Label::new("foobar").with_auto_id(),
                Some(CrossAxisAlignment::Start),
            )
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "stack_row_cross_axis_start");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "stack_row_cross_axis_center");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "stack_row_cross_axis_end");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "stack_row_cross_axis_baseline");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "stack_row_cross_axis_fill");
    }

    #[test]
    fn stack_row_main_axis_snapshots() {
        let widget = Stack::row()
            .with_child(Label::new("hello").with_auto_id(), None)
            .with_child(Label::new("world").with_auto_id(), None)
            .with_child(
                Label::new("foobar").with_auto_id(),
                Some(CrossAxisAlignment::Start),
            )
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_start");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_center");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_end");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_spaceBetween");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "stack_row_main_axis_spaceAround");
    }

    #[test]
    fn stack_col_cross_axis_snapshots() {
        let widget = Stack::column()
            .with_child(Label::new("hello").with_auto_id(), None)
            .with_child(Label::new("world").with_auto_id(), None)
            .with_child(
                Label::new("foobar").with_auto_id(),
                Some(CrossAxisAlignment::Start),
            )
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "stack_col_cross_axis_start");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "stack_col_cross_axis_center");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "stack_col_cross_axis_end");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "stack_col_cross_axis_baseline");

        harness.edit_root_widget(|mut stack| {
            Stack::set_cross_axis_alignment(&mut stack, CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "stack_col_cross_axis_fill");
    }

    #[test]
    fn stack_col_main_axis_snapshots() {
        let widget = Stack::column()
            .with_child(Label::new("hello").with_auto_id(), None)
            .with_child(Label::new("world").with_auto_id(), None)
            .with_child(
                Label::new("foobar").with_auto_id(),
                Some(CrossAxisAlignment::Start),
            )
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_start");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_center");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_end");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_spaceBetween");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut stack| {
            Stack::set_main_axis_alignment(&mut stack, MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "stack_col_main_axis_spaceAround");
    }

    #[test]
    fn get_stack_child() {
        let widget = Stack::column()
            .with_child(Label::new("hello").with_auto_id(), None)
            .with_child(Label::new("world").with_auto_id(), None)
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);
        harness.edit_root_widget(|mut stack| {
            let mut child = Stack::child_mut(&mut stack, 1);
            assert_eq!(
                child
                    .try_downcast::<Label>()
                    .unwrap()
                    .widget
                    .text()
                    .to_string(),
                "world"
            );
        });

        // TODO - test out-of-bounds access?
    }
}
