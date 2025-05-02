// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::trace_span;

use crate::core::{
    AccessCtx, BoxConstraints, LayoutCtx, PaintCtx, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::Size;
use crate::properties::types::Alignment;
use crate::vello::Scene;

struct Child {
    widget: WidgetPod<dyn Widget>,
    alignment: ChildAlignment,
}

/// An option specifying how a child widget is aligned within a [`ZStack`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChildAlignment {
    /// Specifies that the child should use the global alignment as specified by the parent [`ZStack`] widget.
    ParentAligned,
    /// Specifies that the child should override the global alignment specified by the parent [`ZStack`] widget.
    SelfAligned(Alignment),
}

/// A widget container that lays the child widgets on top of each other.
///
/// The alignment of how the children are placed can be specified globally using [`with_alignment`][Self::with_alignment].
/// Each child can additionally override the global alignment using [`ChildAlignment::SelfAligned`].
///
#[doc = crate::include_screenshot!("widget/screenshots/masonry__widget__zstack__tests__zstack_alignment_default.png", "Red foreground widget on top of blue background widget.")]
#[derive(Default)]
pub struct ZStack {
    children: Vec<Child>,
    alignment: Alignment,
}

// --- MARK: CHILD ---
impl From<Alignment> for ChildAlignment {
    fn from(value: Alignment) -> Self {
        Self::SelfAligned(value)
    }
}

impl Child {
    fn new(widget: WidgetPod<dyn Widget>, alignment: ChildAlignment) -> Self {
        Self { widget, alignment }
    }

    fn update_alignment(&mut self, alignment: ChildAlignment) {
        self.alignment = alignment;
    }
}

// --- MARK: IMPL ZSTACK ---
impl ZStack {
    /// Constructs a new empty `ZStack` widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes the alignment of the children.
    pub fn with_alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Appends a child widget to the `ZStack`.
    /// The child are placed back to front, in the order they are added.
    pub fn with_child(self, child: impl Widget, alignment: impl Into<ChildAlignment>) -> Self {
        self.with_child_pod(WidgetPod::new(child).erased(), alignment)
    }

    /// Appends a child widget with a given `id` to the `ZStack`.
    pub fn with_child_id(
        self,
        child: impl Widget,
        id: WidgetId,
        alignment: impl Into<ChildAlignment>,
    ) -> Self {
        self.with_child_pod(WidgetPod::new_with_id(child, id).erased(), alignment)
    }

    /// Appends a child widget pod to the `ZStack`.
    ///
    /// See also [`Self::with_child`] if the widget is not already wrapped in a [`WidgetPod`].
    pub fn with_child_pod(
        mut self,
        child: WidgetPod<dyn Widget>,
        alignment: impl Into<ChildAlignment>,
    ) -> Self {
        let child = Child::new(child, alignment.into());
        self.children.push(child);
        self
    }
}

// --- MARK: WIDGETMUT---
impl ZStack {
    /// Add a child widget to the `ZStack`.
    /// The child are placed back to front, in the order they are added.
    ///
    /// See also [`with_child`][Self::with_child].
    pub fn add_child(
        this: &mut WidgetMut<'_, Self>,
        child: impl Widget,
        alignment: impl Into<ChildAlignment>,
    ) {
        let child_pod: WidgetPod<dyn Widget> = WidgetPod::new(child).erased();
        Self::insert_child_pod(this, child_pod, alignment);
    }

    /// Add a child widget with a given `id` to the `ZStack`.
    ///
    /// See [`Self::add_child`] for more details.
    pub fn add_child_id(
        this: &mut WidgetMut<'_, Self>,
        child: impl Widget,
        id: WidgetId,
        alignment: impl Into<ChildAlignment>,
    ) {
        let child_pod: WidgetPod<dyn Widget> = WidgetPod::new_with_id(child, id).erased();
        Self::insert_child_pod(this, child_pod, alignment);
    }

    /// Add a child widget to the `ZStack`.
    pub fn insert_child_pod(
        this: &mut WidgetMut<'_, Self>,
        widget: WidgetPod<dyn Widget>,
        alignment: impl Into<ChildAlignment>,
    ) {
        let child = Child::new(widget, alignment.into());
        this.widget.children.push(child);
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// Remove a child from the `ZStack`.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
        this.ctx.request_layout();
    }

    /// Get a mutable reference to a child of the `ZStack`.
    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> Option<WidgetMut<'t, dyn Widget>> {
        let child = &mut this.widget.children[idx].widget;
        Some(this.ctx.get_mut(child))
    }

    /// Change the alignment of the `ZStack`.
    ///
    /// See also [`with_alignment`][Self::with_alignment].
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: impl Into<Alignment>) {
        this.widget.alignment = alignment.into();
        this.ctx.request_layout();
    }

    /// Change the alignment of a child of the `ZStack`.
    pub fn update_child_alignment(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        alignment: impl Into<ChildAlignment>,
    ) {
        let child = &mut this.widget.children[idx];
        child.update_alignment(alignment.into());
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for ZStack {
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // First pass: calculate the smallest bounds needed to layout the children.
        let mut max_size = bc.min();
        let loosened_bc = bc.loosen();
        for child in &mut self.children {
            let child_size = ctx.run_layout(&mut child.widget, &loosened_bc);

            max_size.width = child_size.width.max(max_size.width);
            max_size.height = child_size.height.max(max_size.height);
        }

        // Second pass: place the children given the calculated max_size bounds.
        for child in &mut self.children {
            let child_size = ctx.child_size(&child.widget);
            let child_alignment = match child.alignment {
                ChildAlignment::SelfAligned(alignment) => alignment,
                ChildAlignment::ParentAligned => self.alignment,
            };
            let origin = child_alignment.resolve_pos(child_size, max_size);

            ctx.place_child(&mut child.widget, origin);
        }

        max_size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        for child in self.children.iter_mut().map(|x| &mut x.widget) {
            ctx.register_child(child);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .map(|child| &child.widget)
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> tracing::Span {
        trace_span!("ZStack", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use vello::peniko::color::palette;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::properties::types::{HorizontalAlignment, VerticalAlignment};
    use crate::testing::TestHarness;
    use crate::widgets::{Label, SizedBox};

    #[test]
    fn zstack_alignments_parent_aligned() {
        let widget = ZStack::new()
            .with_child(
                SizedBox::new(Label::new("Background"))
                    .width(200.)
                    .height(100.)
                    .background(palette::css::BLUE)
                    .border(palette::css::TEAL, 2.),
                ChildAlignment::ParentAligned,
            )
            .with_child(
                SizedBox::new(Label::new("Foreground"))
                    .background(palette::css::RED)
                    .border(palette::css::PINK, 2.),
                ChildAlignment::ParentAligned,
            );

        let mut harness = TestHarness::create(widget);
        assert_render_snapshot!(harness, "zstack_alignment_default");

        let vertical_cases = [
            ("top", VerticalAlignment::Top),
            ("center", VerticalAlignment::Center),
            ("bottom", VerticalAlignment::Bottom),
        ];

        let horizontal_cases = [
            ("left", HorizontalAlignment::Left),
            ("center", HorizontalAlignment::Center),
            ("right", HorizontalAlignment::Right),
        ];

        let all_cases = vertical_cases
            .into_iter()
            .flat_map(|vert| horizontal_cases.map(|hori| (vert, hori)));

        for (vertical, horizontal) in all_cases {
            harness.edit_root_widget(|mut zstack| {
                let mut zstack = zstack.downcast::<ZStack>();
                ZStack::set_alignment(&mut zstack, (vertical.1, horizontal.1));
            });
            assert_render_snapshot!(
                harness,
                &format!("zstack_alignment_{}_{}", vertical.0, horizontal.0)
            );
        }
    }

    #[test]
    fn zstack_alignments_self_aligned() {
        let widget = ZStack::new()
            .with_alignment(Alignment::Center)
            .with_child(Label::new("ParentAligned"), ChildAlignment::ParentAligned)
            .with_child(Label::new("TopLeft"), Alignment::TopLeft)
            .with_child(Label::new("TopRight"), Alignment::TopRight)
            .with_child(Label::new("BottomLeft"), Alignment::BottomLeft)
            .with_child(Label::new("BottomRight"), Alignment::BottomRight);

        let mut harness = TestHarness::create(widget);
        assert_render_snapshot!(harness, "zstack_alignments_self_aligned");
    }
}
