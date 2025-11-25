// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::trace_span;
use vello::Scene;
use vello::kurbo::{Rect, Size};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::types::UnitPoint;
use crate::util::include_screenshot;

struct Child {
    widget: WidgetPod<dyn Widget>,
    alignment: ChildAlignment,
}

/// An option specifying how a child widget is aligned within a [`ZStack`].
#[derive(Clone, Copy, PartialEq)]
pub enum ChildAlignment {
    /// Specifies that the child should use the global alignment as specified by the parent [`ZStack`] widget.
    ParentAligned,
    /// Specifies that the child should override the global alignment specified by the parent [`ZStack`] widget.
    SelfAligned(UnitPoint),
}

/// A widget container that lays the child widgets on top of each other.
///
/// The alignment of how the children are placed can be specified globally using [`with_alignment`][Self::with_alignment].
/// Each child can additionally override the global alignment using [`ChildAlignment::SelfAligned`].
///
#[doc = include_screenshot!("zstack_alignment_default.png", "Red foreground widget on top of blue background widget.")]
pub struct ZStack {
    children: Vec<Child>,
    alignment: UnitPoint,
}

impl Default for ZStack {
    fn default() -> Self {
        Self {
            children: Vec::default(),
            alignment: UnitPoint::CENTER,
        }
    }
}

// --- MARK: IMPL ALIGNMENTS

impl From<UnitPoint> for ChildAlignment {
    fn from(value: UnitPoint) -> Self {
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

// --- MARK: IMPL ZSTACK
impl ZStack {
    /// Constructs a new empty `ZStack` widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes the alignment of the children.
    pub fn with_alignment(mut self, alignment: impl Into<UnitPoint>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Appends a child widget to the `ZStack`.
    /// The child are placed back to front, in the order they are added.
    pub fn with_child(
        mut self,
        child: NewWidget<impl Widget + ?Sized>,
        alignment: impl Into<ChildAlignment>,
    ) -> Self {
        let child = Child::new(child.erased().to_pod(), alignment.into());
        self.children.push(child);
        self
    }
}

// --- MARK: WIDGETMUT
impl ZStack {
    /// Add a child widget to the `ZStack`.
    /// The child are placed back to front, in the order they are added.
    ///
    /// See also [`with_child`][Self::with_child].
    pub fn insert_child(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        alignment: impl Into<ChildAlignment>,
    ) {
        let child = Child::new(child.erased().to_pod(), alignment.into());
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Remove a child from the `ZStack`.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
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
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: impl Into<UnitPoint>) {
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

// --- MARK: IMPL WIDGET
impl Widget for ZStack {
    type Action = NoAction;

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
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
            let end = max_size - child_size;

            let child_alignment = match child.alignment {
                ChildAlignment::SelfAligned(alignment) => alignment,
                ChildAlignment::ParentAligned => self.alignment,
            };

            let origin = child_alignment.resolve(Rect::new(0., 0., end.width, end.height));

            ctx.place_child(&mut child.widget, origin);
        }

        max_size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut().map(|x| &mut x.widget) {
            ctx.register_child(child);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
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
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("ZStack", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use vello::peniko::color::palette;

    use super::*;
    use crate::core::Properties;
    use crate::properties::types::AsUnit;
    use crate::properties::{Background, BorderColor, BorderWidth};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Label, SizedBox};

    #[test]
    fn zstack_alignments_parent_aligned() {
        let mut bg_props = Properties::new();
        bg_props.insert(Background::Color(palette::css::BLUE));
        bg_props.insert(BorderColor::new(palette::css::TEAL));
        bg_props.insert(BorderWidth::all(2.0));

        let mut fg_props = Properties::new();
        fg_props.insert(Background::Color(palette::css::RED));
        fg_props.insert(BorderColor::new(palette::css::PINK));
        fg_props.insert(BorderWidth::all(2.0));

        let widget = ZStack::new()
            .with_child(
                NewWidget::new_with_props(
                    SizedBox::new(Label::new("Background").with_auto_id())
                        .width(200.px())
                        .height(100.px()),
                    bg_props,
                ),
                ChildAlignment::ParentAligned,
            )
            .with_child(
                NewWidget::new_with_props(
                    SizedBox::new(Label::new("Foreground").with_auto_id()),
                    fg_props,
                ),
                ChildAlignment::ParentAligned,
            )
            .with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);
        assert_render_snapshot!(harness, "zstack_alignment_default");

        let all_cases = [
            (UnitPoint::TOP_LEFT, "top_left"),
            (UnitPoint::TOP, "top_center"),
            (UnitPoint::TOP_RIGHT, "top_right"),
            (UnitPoint::LEFT, "center_left"),
            (UnitPoint::CENTER, "center_center"),
            (UnitPoint::RIGHT, "center_right"),
            (UnitPoint::BOTTOM_LEFT, "bottom_left"),
            (UnitPoint::BOTTOM, "bottom_center"),
            (UnitPoint::BOTTOM_RIGHT, "bottom_right"),
            (UnitPoint::new(0.2, 1.0), "bottom_leftish"),
        ];

        for (align, name) in all_cases {
            harness.edit_root_widget(|mut zstack| {
                ZStack::set_alignment(&mut zstack, align);
            });
            assert_render_snapshot!(harness, &format!("zstack_alignment_{name}"));
        }
    }

    #[test]
    fn zstack_alignments_self_aligned() {
        let widget = ZStack::new()
            .with_alignment(UnitPoint::CENTER)
            .with_child(
                Label::new("ParentAligned").with_auto_id(),
                ChildAlignment::ParentAligned,
            )
            .with_child(Label::new("TopLeft").with_auto_id(), UnitPoint::TOP_LEFT)
            .with_child(Label::new("TopRight").with_auto_id(), UnitPoint::TOP_RIGHT)
            .with_child(
                Label::new("BottomLeft").with_auto_id(),
                UnitPoint::BOTTOM_LEFT,
            )
            .with_child(
                Label::new("BottomRight").with_auto_id(),
                UnitPoint::BOTTOM_RIGHT,
            )
            .with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);
        assert_render_snapshot!(harness, "zstack_alignments_self_aligned");
    }
}
