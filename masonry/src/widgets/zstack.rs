// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::trace_span;
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::util::include_screenshot;

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
#[doc = include_screenshot!("zstack_alignment_default.png", "Red foreground widget on top of blue background widget.")]
#[derive(Default)]
pub struct ZStack {
    children: Vec<Child>,
    alignment: Alignment,
}

/// Alignment describes the position of a view laid on top of another view.
///
/// See also [`VerticalAlignment`] and [`HorizontalAlignment`] for describing only a single axis.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Align to the top left corner.
    TopLeft,
    /// Align to the center of the top edge.
    Top,
    /// Align to the top right corner.
    TopRight,
    /// Align to the center of the left edge.
    Left,
    /// Align to the center.
    #[default]
    Center,
    /// Align to the center of the right edge.
    Right,
    /// Align to the bottom left corner.
    BottomLeft,
    /// Align to the center of the bottom edge.
    Bottom,
    /// Align to the bottom right corner.
    BottomRight,
}

/// Describes the vertical position of a view laid on top of another view.
///
/// See also [`Alignment`].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    /// Align to the top edge.
    Top,
    /// Align to the center.
    #[default]
    Center,
    /// Align to the bottom edge.
    Bottom,
}

/// Describes the horizontal position of a view laid on top of another view.
///
/// See also [`Alignment`].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    /// Align to the left edge.
    Left,
    #[default]
    /// Align to the center.
    Center,
    /// Align to the right edge.
    Right,
}

// --- MARK: IMPL ALIGNMENTS

impl Alignment {
    /// Constructs a new `Alignment` from a [vertical][`VerticalAlignment`] and [horizontal][`HorizontalAlignment`] alignment.
    pub fn new(vertical: VerticalAlignment, horizontal: HorizontalAlignment) -> Self {
        match (vertical, horizontal) {
            (VerticalAlignment::Top, HorizontalAlignment::Left) => Self::TopLeft,
            (VerticalAlignment::Top, HorizontalAlignment::Center) => Self::Top,
            (VerticalAlignment::Top, HorizontalAlignment::Right) => Self::TopRight,
            (VerticalAlignment::Center, HorizontalAlignment::Left) => Self::Left,
            (VerticalAlignment::Center, HorizontalAlignment::Center) => Self::Center,
            (VerticalAlignment::Center, HorizontalAlignment::Right) => Self::Right,
            (VerticalAlignment::Bottom, HorizontalAlignment::Left) => Self::BottomLeft,
            (VerticalAlignment::Bottom, HorizontalAlignment::Center) => Self::Bottom,
            (VerticalAlignment::Bottom, HorizontalAlignment::Right) => Self::BottomRight,
        }
    }

    /// Gets the vertical component of the alignment.
    pub fn vertical(self) -> VerticalAlignment {
        match self {
            Self::Center | Self::Left | Self::Right => VerticalAlignment::Center,
            Self::Top | Self::TopLeft | Self::TopRight => VerticalAlignment::Top,
            Self::Bottom | Self::BottomLeft | Self::BottomRight => VerticalAlignment::Bottom,
        }
    }

    /// Gets the horizontal component of the alignment.
    pub fn horizontal(self) -> HorizontalAlignment {
        match self {
            Self::Center | Self::Top | Self::Bottom => HorizontalAlignment::Center,
            Self::Left | Self::TopLeft | Self::BottomLeft => HorizontalAlignment::Left,
            Self::Right | Self::TopRight | Self::BottomRight => HorizontalAlignment::Right,
        }
    }
}

impl From<Alignment> for VerticalAlignment {
    fn from(value: Alignment) -> Self {
        value.vertical()
    }
}

impl From<Alignment> for HorizontalAlignment {
    fn from(value: Alignment) -> Self {
        value.horizontal()
    }
}

impl From<(VerticalAlignment, HorizontalAlignment)> for Alignment {
    fn from((vertical, horizontal): (VerticalAlignment, HorizontalAlignment)) -> Self {
        Self::new(vertical, horizontal)
    }
}

impl From<VerticalAlignment> for Alignment {
    fn from(vertical: VerticalAlignment) -> Self {
        Self::new(vertical, HorizontalAlignment::Center)
    }
}

impl From<HorizontalAlignment> for Alignment {
    fn from(horizontal: HorizontalAlignment) -> Self {
        Self::new(VerticalAlignment::Center, horizontal)
    }
}

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

// --- MARK: IMPL ZSTACK
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
    pub fn with_child(
        mut self,
        // TODO: +?Sized
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

// --- MARK: IMPL WIDGET
impl Widget for ZStack {
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
            let end = Point::new(end.width, end.height);

            let center = Point::new(end.x / 2., end.y / 2.);

            let child_alignment = match child.alignment {
                ChildAlignment::SelfAligned(alignment) => alignment,
                ChildAlignment::ParentAligned => self.alignment,
            };

            let origin = match child_alignment {
                Alignment::TopLeft => Point::ZERO,
                Alignment::Top => Point::new(center.x, 0.),
                Alignment::TopRight => Point::new(end.x, 0.),
                Alignment::Left => Point::new(0., center.y),
                Alignment::Center => center,
                Alignment::Right => Point::new(end.x, center.y),
                Alignment::BottomLeft => Point::new(0., end.y),
                Alignment::Bottom => Point::new(center.x, end.y),
                Alignment::BottomRight => end,
            };

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
    use masonry_core::core::Properties;
    use vello::peniko::color::palette;

    use super::*;
    use crate::properties::{Background, BorderColor, BorderWidth};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
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
                        .width(200.)
                        .height(100.),
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
            );

        let mut harness = TestHarness::create(default_property_set(), widget);
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
            .with_child(
                Label::new("ParentAligned").with_auto_id(),
                ChildAlignment::ParentAligned,
            )
            .with_child(Label::new("TopLeft").with_auto_id(), Alignment::TopLeft)
            .with_child(Label::new("TopRight").with_auto_id(), Alignment::TopRight)
            .with_child(
                Label::new("BottomLeft").with_auto_id(),
                Alignment::BottomLeft,
            )
            .with_child(
                Label::new("BottomRight").with_auto_id(),
                Alignment::BottomRight,
            );

        let mut harness = TestHarness::create(default_property_set(), widget);
        assert_render_snapshot!(harness, "zstack_alignments_self_aligned");
    }
}
