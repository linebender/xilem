// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, CollectionWidget, HasProperty, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::kurbo::{Affine, Axis, Line, Point, Size, Stroke};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::util::{fill, stroke};

// TODO - Rename "active" widget to "visible" widget?
// Active already means something else.

/// A widget that displays only one of its children at a time.
///
/// This is useful for switching between multiple views while keeping
/// state loaded, such as in a tab stack.
///
/// The indexed stack acts as a simple container around the active child.
/// If there is no active child, it acts like a leaf node with no content.
#[doc = concat!(
    "![Indexed stack element showing only the fourth element in its children](",
    include_doc_path!("screenshots/indexed_stack_builder_new_widget.png"),
    ")",
)]
#[derive(Default)]
pub struct IndexedStack {
    children: Vec<WidgetPod<dyn Widget>>,
    // Note: active_child must be 0 if there are no children
    active_child: usize,
}

// --- MARK: BUILDERS
impl IndexedStack {
    /// Creates a new stack with no children.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style method to add a child widget.
    pub fn with(mut self, child: NewWidget<impl Widget + ?Sized>) -> Self {
        self.children.push(child.erased().to_pod());
        self
    }

    /// Builder-style method to set the active child.
    ///
    /// Index must be a valid index into the stack's children, or 0
    /// if there are no children.
    pub fn with_active_child(mut self, idx: usize) -> Self {
        if self.children.is_empty() {
            assert!(
                idx == 0,
                "Called set_active on empty IndexedStack with non-zero index {idx}"
            );
        } else {
            assert!(
                idx < self.children.len(),
                "Called set_active with invalid index {idx}"
            );
        }

        self.active_child = idx;
        self
    }
}

// --- MARK: METHODS
impl IndexedStack {
    /// Returns the index of the currently active child.
    pub fn active_child(&self) -> usize {
        self.active_child
    }
}

// --- MARK: WIDGETMUT
impl IndexedStack {
    /// Sets the active child.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    /// If there are no children, the index 0 is accepted.
    pub fn set_active_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        if this.widget.children.is_empty() {
            assert!(
                idx == 0,
                "Called set_active_child on empty IndexedStack with non-zero index {idx}"
            );
        } else {
            assert!(
                idx < this.widget.children.len(),
                "Called set_active_child with invalid index {idx}"
            );
        }
        this.widget.active_child = idx;
        this.ctx.request_layout();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<()> for IndexedStack {
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
        let child = &mut this.widget.children[idx];
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        _params: impl Into<()>,
    ) {
        this.widget.children.push(child.erased().to_pod());
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
        _params: impl Into<()>,
    ) {
        this.widget.children.insert(idx, child.erased().to_pod());
        if this.widget.active_child >= idx {
            // adjust index to keep the same widget active
            this.widget.active_child += 1;
        }
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
        _params: impl Into<()>,
    ) {
        let old_child = std::mem::replace(&mut this.widget.children[idx], child.erased().to_pod());
        this.ctx.remove_child(old_child);
    }

    /// Not applicable.
    fn set_params(_this: &mut WidgetMut<'_, Self>, _idx: usize, _params: impl Into<()>) {}

    /// Swaps the index of two children.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        this.widget.children.swap(a, b);
        this.ctx.children_changed();
    }

    /// Removes the child at the given index.
    ///
    /// If the active child is removed, the first child in the stack will be selected as active.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        if idx == this.widget.active_child {
            // This is valid even if we are removing the last child,
            // since `active_child` must be 0 in that case
            this.widget.active_child = 0;
        } else if this.widget.active_child > idx {
            // correct the index to prevent the active element changing
            this.widget.active_child -= 1;
        }
        this.ctx.remove_child(child);
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child);
        }
        this.widget.active_child = 0; // 0 is valid for the childrenless case
    }
}

impl HasProperty<Background> for IndexedStack {}
impl HasProperty<BorderColor> for IndexedStack {}
impl HasProperty<BorderWidth> for IndexedStack {}
impl HasProperty<CornerRadius> for IndexedStack {}

// --- MARK: IMPL WIDGET
impl Widget for IndexedStack {
    type Action = NoAction;

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
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let border_length = border.length(axis).dp(scale);
        let padding_length = padding.length(axis).dp(scale);

        let child_length = if !self.children.is_empty() {
            let cross = axis.cross();
            let cross_space = cross_length.map(|cross_length| {
                let cross_border_length = border.length(cross).dp(scale);
                let cross_padding_length = padding.length(cross).dp(scale);
                (cross_length - cross_border_length - cross_padding_length).max(0.)
            });

            let auto_length = len_req.reduce(border_length + padding_length).into();
            let context_size = LayoutSize::maybe(cross, cross_space);

            ctx.compute_length(
                &mut self.children[self.active_child],
                auto_length,
                context_size,
                axis,
                cross_space,
            )
        } else {
            0.
        };

        child_length + border_length + padding_length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // There's nothing to lay out if we don't have any children
        if self.children.is_empty() {
            return;
        }

        // TODO: move set_stashed to a different layout pass when possible
        for (idx, child) in self.children.iter_mut().enumerate() {
            ctx.set_stashed(child, idx != self.active_child);
        }

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        let child_size = ctx.compute_size(
            &mut self.children[self.active_child],
            SizeDef::fit(space),
            space.into(),
        );
        ctx.run_layout(&mut self.children[self.active_child], child_size);

        let child_origin = Point::ORIGIN;
        let child_origin = border.origin_down(child_origin, scale);
        let child_origin = padding.origin_down(child_origin, scale);
        ctx.place_child(&mut self.children[self.active_child], child_origin);

        let child_baseline = ctx.child_baseline_offset(&self.children[self.active_child]);
        let child_baseline = border.baseline_up(child_baseline, scale);
        let child_baseline = padding.baseline_up(child_baseline, scale);
        let child_bottom = child_origin.y + child_size.height;
        let bottom_gap = size.height - child_bottom;
        ctx.set_baseline_offset(child_baseline + bottom_gap);
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
        self.children.iter().map(WidgetPod::id).collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("IndexedStack", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::Dimensions;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Button;

    #[test]
    fn test_indexed_stack_basics() {
        let widget = IndexedStack::new().with_auto_id();
        let window_size = Size::new(50.0, 50.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "indexed_stack_empty");

        harness.edit_root_widget(|mut stack| {
            IndexedStack::add(
                &mut stack,
                Button::with_text("A").with_props(Dimensions::STRETCH),
                (),
            );
        });
        assert_render_snapshot!(harness, "indexed_stack_single");

        harness.edit_root_widget(|mut stack| {
            IndexedStack::add(
                &mut stack,
                Button::with_text("B").with_props(Dimensions::STRETCH),
                (),
            );
            IndexedStack::add(
                &mut stack,
                Button::with_text("C").with_props(Dimensions::STRETCH),
                (),
            );
            IndexedStack::add(
                &mut stack,
                Button::with_text("D").with_props(Dimensions::STRETCH),
                (),
            );
        });
        assert_render_snapshot!(harness, "indexed_stack_single"); // the active child should not change

        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_active_child(&mut stack, 2);
        });
        assert_render_snapshot!(harness, "indexed_stack_many_2");
    }

    #[test]
    fn test_widget_removal_and_modification() {
        let widget = IndexedStack::new()
            .with(Button::with_text("A").with_props(Dimensions::STRETCH))
            .with(Button::with_text("B").with_props(Dimensions::STRETCH))
            .with(Button::with_text("C").with_props(Dimensions::STRETCH))
            .with_active_child(1)
            .with_auto_id();
        let window_size = Size::new(50.0, 50.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "indexed_stack_initial_builder");

        // Remove the first (inactive) widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::remove(&mut stack, 0);
        });
        assert_render_snapshot!(harness, "indexed_stack_initial_builder"); // Should not be changed

        // Now remove the active widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::remove(&mut stack, 0);
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_removed_widget");

        // Add another widget at the end
        harness.edit_root_widget(|mut stack| {
            IndexedStack::add(
                &mut stack,
                Button::with_text("D").with_props(Dimensions::STRETCH),
                (),
            );
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_removed_widget"); // Should not change

        // Make it active
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_active_child(&mut stack, 1);
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget");

        // Insert back the first two at the start
        harness.edit_root_widget(|mut stack| {
            IndexedStack::insert(
                &mut stack,
                0,
                Button::with_text("A").with_props(Dimensions::STRETCH),
                (),
            );
            IndexedStack::insert(
                &mut stack,
                1,
                Button::with_text("B").with_props(Dimensions::STRETCH),
                (),
            );
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget"); // Should not change

        // Reset original active index
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_active_child(&mut stack, 1);
        });
        assert_render_snapshot!(harness, "indexed_stack_initial_builder");

        // Change the active widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set(
                &mut stack,
                1,
                Button::with_text("D").with_props(Dimensions::STRETCH),
                (),
            );
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget");
    }
}
