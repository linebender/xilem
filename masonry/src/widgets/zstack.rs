// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::trace_span;
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, CollectionWidget, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Rect, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef, UnitPoint};

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

/// A widget container that lays the child widgets on top of each other.
///
/// The alignment of how the children are placed can be specified globally using [`with_alignment`][Self::with_alignment].
/// Each child can additionally override the global alignment using [`ChildAlignment::SelfAligned`].
///
#[doc = concat!(
    "![Red foreground widget on top of blue background widget](",
    include_doc_path!("screenshots/zstack_alignment_default.png"),
    ")",
)]
pub struct ZStack {
    children: Vec<Child>,
    alignment: UnitPoint,
}

// --- MARK: DEFAULT
impl Default for ZStack {
    fn default() -> Self {
        Self {
            children: Vec::default(),
            alignment: UnitPoint::CENTER,
        }
    }
}

// --- MARK: BUILDERS
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
    pub fn with(
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
    /// Changes the alignment of the `ZStack`.
    ///
    /// See also [`with_alignment`][Self::with_alignment].
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: impl Into<UnitPoint>) {
        this.widget.alignment = alignment.into();
        this.ctx.request_layout();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<ChildAlignment> for ZStack {
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
        let child = &mut this.widget.children[idx].widget;
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<ChildAlignment>,
    ) {
        let child = Child::new(child.erased().to_pod(), params.into());
        this.widget.children.push(child);
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
        params: impl Into<ChildAlignment>,
    ) {
        let child = Child::new(child.erased().to_pod(), params.into());
        this.widget.children.insert(idx, child);
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
        params: impl Into<ChildAlignment>,
    ) {
        let child = Child::new(child.erased().to_pod(), params.into());
        let old_child = std::mem::replace(&mut this.widget.children[idx], child);
        this.ctx.remove_child(old_child.widget);
    }

    /// Sets the child alignment at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<ChildAlignment>) {
        let child = &mut this.widget.children[idx];
        child.update_alignment(params.into());
        this.ctx.request_layout();
    }

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
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child.widget);
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for ZStack {
    type Action = NoAction;

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        let mut length: f64 = 0.;
        for child in &mut self.children {
            let child_length = ctx.compute_length(
                &mut child.widget,
                auto_length,
                context_size,
                axis,
                cross_length,
            );
            length = length.max(child_length);
        }

        length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let context_size = size.into();
        let auto_size = SizeDef::fit(size);
        for child in &mut self.children {
            let child_size = ctx.compute_size(&mut child.widget, auto_size, context_size);
            ctx.run_layout(&mut child.widget, child_size);

            let child_alignment = match child.alignment {
                ChildAlignment::SelfAligned(alignment) => alignment,
                ChildAlignment::ParentAligned => self.alignment,
            };

            let extra_width = (size.width - child_size.width).max(0.);
            let extra_height = (size.height - child_size.height).max(0.);
            let child_origin =
                child_alignment.resolve(Rect::new(0., 0., extra_width, extra_height));
            ctx.place_child(&mut child.widget, child_origin);
        }
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
    use super::*;
    use crate::core::PropertySet;
    use crate::layout::AsUnit;
    use crate::peniko::color::palette;
    use crate::properties::{Background, BorderColor, BorderWidth};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Label, SizedBox};

    #[test]
    fn zstack_alignments_parent_aligned() {
        let mut bg_props = PropertySet::new();
        bg_props.insert(Background::Color(palette::css::BLUE));
        bg_props.insert(BorderColor::new(palette::css::TEAL));
        bg_props.insert(BorderWidth::all(2.0));

        let mut fg_props = PropertySet::new();
        fg_props.insert(Background::Color(palette::css::RED));
        fg_props.insert(BorderColor::new(palette::css::PINK));
        fg_props.insert(BorderWidth::all(2.0));

        let widget = ZStack::new()
            .with(
                NewWidget::new_with_props(
                    SizedBox::new(Label::new("Background").with_auto_id())
                        .width(200.px())
                        .height(100.px()),
                    bg_props,
                ),
                ChildAlignment::ParentAligned,
            )
            .with(
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
            .with(
                Label::new("ParentAligned").with_auto_id(),
                ChildAlignment::ParentAligned,
            )
            .with(Label::new("TopLeft").with_auto_id(), UnitPoint::TOP_LEFT)
            .with(Label::new("TopRight").with_auto_id(), UnitPoint::TOP_RIGHT)
            .with(
                Label::new("BottomLeft").with_auto_id(),
                UnitPoint::BOTTOM_LEFT,
            )
            .with(
                Label::new("BottomRight").with_auto_id(),
                UnitPoint::BOTTOM_RIGHT,
            )
            .with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);
        assert_render_snapshot!(harness, "zstack_alignments_self_aligned");
    }
}
