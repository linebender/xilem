// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::HasProperty;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Line, Point, Size, Stroke};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::util::{debug_panic, fill, include_screenshot, stroke};

/// A widget that displays only one of its children at a time.
///
/// This is useful for switching between multiple views while keeping
/// state loaded, such as in a tab stack.
///
/// The indexed stack acts as a simple container around the active child.
/// If there is no active child, it acts like a leaf node, and takes up
/// the minimum space.
#[doc = include_screenshot!("indexed_stack_builder_new_widget.png", "Indexed stack element showing only the fourth element in its children.")]
#[derive(Default)]
pub struct IndexedStack {
    children: Vec<WidgetPod<dyn Widget>>,
    // Note: active_child must be 0 if there are no children
    active_child: usize,
}

// --- MARK: BUILDERS
impl IndexedStack {
    /// Create a new stack with no children.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style method to add a child widget.
    pub fn with_child(mut self, child: NewWidget<impl Widget + ?Sized>) -> Self {
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
                "Called set_active_child on empty IndexedStack with non-zero index {idx}"
            );
        } else {
            assert!(
                idx < self.children.len(),
                "Called set_active_child with invalid index {idx}"
            );
        }

        self.active_child = idx;
        self
    }
}

// --- MARK: METHODS
impl IndexedStack {
    /// Returns the number of children in this stack.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns true if there are no children in this stack.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the index of the currently active child.
    pub fn active_child_index(&self) -> usize {
        self.active_child
    }
}

// --- MARK: WIDGETMUT
impl IndexedStack {
    /// Add a child widget to the end of the stack.
    ///
    /// See also [`with_child`](IndexedStack::with_child).
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        this.widget.children.push(child.erased().to_pod());
        this.ctx.children_changed();
    }

    /// Insert a child widget at the given index.
    ///
    /// This lets you control the order of the children stored by the indexed stack.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
    ) {
        this.widget.children.insert(idx, child.erased().to_pod());
        if this.widget.active_child >= idx {
            // adjust index to keep the same widget active
            this.widget.active_child += 1;
        }
        this.ctx.children_changed();
    }

    /// Replace the child widget at the given index with a new one.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn set_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
    ) {
        let old_child = std::mem::replace(&mut this.widget.children[idx], child.erased().to_pod());
        this.ctx.remove_child(old_child);
    }

    /// Change the active child.
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

    /// Get a mutable reference to the child at `idx`.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.children[idx];
        this.ctx.get_mut(child)
    }

    /// Removes a child widget at the given index. If the active is removed,
    /// the first child in the stack will be selected as active.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
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
}

impl HasProperty<Background> for IndexedStack {}
impl HasProperty<BorderColor> for IndexedStack {}
impl HasProperty<BorderWidth> for IndexedStack {}
impl HasProperty<CornerRadius> for IndexedStack {}
impl HasProperty<Padding> for IndexedStack {}

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

        let origin = Point::ORIGIN;
        let origin = border.place_down(origin);
        let origin = padding.place_down(origin);

        if !(self.children.is_empty() && self.active_child == 0)
            && self.active_child >= self.children.len()
        {
            debug_panic!(
                "IndexedStack active child index ({}) is not within the children vector (len {})",
                self.active_child,
                self.children.len()
            );
        }
        let mut child_size = bc.min();
        for (idx, child) in self.children.iter_mut().enumerate() {
            if idx == self.active_child {
                ctx.set_stashed(child, false);
                let child_bc = bc;
                child_size = ctx.run_layout(child, &child_bc);
                ctx.place_child(child, origin);
            } else {
                // TODO: move set_stashed to a different layout pass when possible,
                ctx.set_stashed(child, true);
            }
        }

        child_size
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
            IndexedStack::add_child(&mut stack, Button::with_text("A").with_auto_id());
        });
        assert_render_snapshot!(harness, "indexed_stack_single");

        harness.edit_root_widget(|mut stack| {
            IndexedStack::add_child(&mut stack, Button::with_text("B").with_auto_id());
            IndexedStack::add_child(&mut stack, Button::with_text("C").with_auto_id());
            IndexedStack::add_child(&mut stack, Button::with_text("D").with_auto_id());
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
            .with_child(Button::with_text("A").with_auto_id())
            .with_child(Button::with_text("B").with_auto_id())
            .with_child(Button::with_text("C").with_auto_id())
            .with_active_child(1)
            .with_auto_id();
        let window_size = Size::new(50.0, 50.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "indexed_stack_initial_builder");

        // Remove the first (inactive) widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::remove_child(&mut stack, 0);
        });
        assert_render_snapshot!(harness, "indexed_stack_initial_builder"); // Should not be changed

        // Now remove the active widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::remove_child(&mut stack, 0);
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_removed_widget");

        // Add another widget at the end
        harness.edit_root_widget(|mut stack| {
            IndexedStack::add_child(&mut stack, Button::with_text("D").with_auto_id());
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_removed_widget"); // Should not change

        // Make it active
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_active_child(&mut stack, 1);
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget");

        // Insert back the first two at the start
        harness.edit_root_widget(|mut stack| {
            IndexedStack::insert_child(&mut stack, 0, Button::with_text("A").with_auto_id());
            IndexedStack::insert_child(&mut stack, 1, Button::with_text("B").with_auto_id());
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget"); // Should not change

        // Reset original active index
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_active_child(&mut stack, 1);
        });
        assert_render_snapshot!(harness, "indexed_stack_initial_builder");

        // Change the active widget
        harness.edit_root_widget(|mut stack| {
            IndexedStack::set_child(&mut stack, 1, Button::with_text("D").with_auto_id());
        });
        assert_render_snapshot!(harness, "indexed_stack_builder_new_widget");
    }
}
