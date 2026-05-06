// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ChildrenIds, CollectionWidget, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PropertiesRef, RegisterCtx, UpdateCtx, UsesProperty, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Size};
use crate::{imaging::Painter, layout::LenReq, properties::Gap};

pub use taffy;

/// A container widget that uses [`taffy`] to facilitate CSS flexbox or grid layout.
pub struct Taffy {
    children: Vec<Child>,
    style: taffy::Style,
    cache: taffy::Cache,
}

enum Child {
    Widget {
        widget: WidgetPod<dyn Widget>,
        style: taffy::Style,
        cache: taffy::Cache,
    },
    Spacer {
        style: taffy::Style,
        cache: taffy::Cache,
    },
}

struct ChildIter(std::ops::Range<usize>);
impl Iterator for ChildIter {
    type Item = taffy::NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(taffy::NodeId::from)
    }
}

enum TaffyCtx<'w, 'a> {
    Layout {
        /// A mutable reference to the widget
        widget: &'w mut Taffy,
        /// A mutable reference to the layout context
        ctx: &'w mut LayoutCtx<'a>,
    },
    Measure {
        /// A mutable reference to the widget
        widget: &'w mut Taffy,
        /// A mutable reference to the layout context
        ctx: &'w mut MeasureCtx<'a>,
    },
}

// --- MARK: BUILDERS
impl Taffy {
    /// Creates a new taffy layout with the given container style.
    pub fn new(style: taffy::Style) -> Self {
        Taffy {
            children: Vec::new(),
            style,
            cache: taffy::Cache::new(),
        }
    }

    /// Builder-style method to add a child widget.
    pub fn with(mut self, child: NewWidget<impl Widget + ?Sized>, style: taffy::Style) -> Self {
        let child = Child::new(child, style);
        self.children.push(child);
        self
    }

    /// Builder-style method to add a spacer.
    pub fn with_spacer(mut self, style: taffy::Style) -> Self {
        let child = Child::spacer(style);
        self.children.push(child);
        self
    }
}

impl Child {
    fn new(widget: NewWidget<impl Widget + ?Sized>, style: impl Into<taffy::Style>) -> Child {
        Child::Widget {
            widget: widget.erased().to_pod(),
            style: style.into(),
            cache: taffy::Cache::new(),
        }
    }

    fn spacer(style: impl Into<taffy::Style>) -> Child {
        Child::Spacer {
            style: style.into(),
            cache: taffy::Cache::new(),
        }
    }
}

impl<'w, 'a> TaffyCtx<'w, 'a> {
    /// Create a new `TaffyLayoutCtx`
    fn new_layout(widget: &'w mut Taffy, ctx: &'w mut LayoutCtx<'a>) -> Self {
        TaffyCtx::Layout { widget, ctx }
    }

    fn new_measure(widget: &'w mut Taffy, ctx: &'w mut MeasureCtx<'a>) -> Self {
        TaffyCtx::Measure { widget, ctx }
    }
}

// --- MARK: WIDGETMUT
impl Taffy {
    /// Sets the taffy parameters for the layout container itself.
    pub fn set_container_style(this: &mut WidgetMut<'_, Self>, style: taffy::Style) {
        this.widget.style = style;
        this.ctx.request_layout();
    }

    /// Adds a new spacer item at the end.
    pub fn add_spacer(this: &mut WidgetMut<'_, Self>, params: impl Into<taffy::Style>) {
        let child = Child::spacer(params);
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Inserts a new spacer item at the given index.
    pub fn insert_spacer(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        params: impl Into<taffy::Style>,
    ) {
        let child = Child::spacer(params);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Sets the item at the given index to be a spacer.
    pub fn set_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<taffy::Style>) {
        let child = Child::spacer(params);
        if let Child::Widget { widget, .. } =
            std::mem::replace(&mut this.widget.children[idx], child)
        {
            this.ctx.remove_child(widget);
        }
        this.ctx.children_changed();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<taffy::Style> for Taffy {
    fn len(&self) -> usize {
        self.children.len()
    }

    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let child = match &mut this.widget.children[idx] {
            Child::Widget { widget, .. } => widget,
            Child::Spacer { .. } => panic!("The provided Taffy idx contains a spacer"),
        };
        this.ctx.get_mut(child)
    }

    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<taffy::Style>,
    ) {
        let child = Child::new(child, params);
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<taffy::Style>,
    ) {
        let child = Child::new(child, params);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<taffy::Style>,
    ) {
        let child = Child::new(child, params);
        if let Child::Widget { widget, .. } =
            std::mem::replace(&mut this.widget.children[idx], child)
        {
            this.ctx.remove_child(widget);
        }
        this.ctx.children_changed();
    }

    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<taffy::Style>) {
        match &mut this.widget.children[idx] {
            Child::Widget { style, .. } => *style = params.into(),
            Child::Spacer { style, .. } => *style = params.into(),
        }
        this.ctx.request_layout();
    }

    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        this.widget.children.swap(a, b);
        this.ctx.children_changed();
    }

    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        if let Child::Widget { widget, .. } = child {
            this.ctx.remove_child(widget);
        } else {
            // We need to explicitly request layout in case of spacer removal
            this.ctx.request_layout();
        }
    }

    fn clear(this: &mut WidgetMut<'_, Self>) {
        if !this.widget.children.is_empty() {
            for child in this.widget.children.drain(..) {
                if let Child::Widget { widget, .. } = child {
                    this.ctx.remove_child(widget);
                }
            }
            // We need to explicitly request layout in case we had any spacers
            this.ctx.request_layout();
        }
    }
}

// --- MARK: OTHER IMPLS
impl<'w, 'a> TaffyCtx<'w, 'a> {
    fn widget(&self) -> &Taffy {
        match self {
            Self::Layout { widget, .. } | Self::Measure { widget, .. } => widget,
        }
    }

    fn widget_mut(&mut self) -> &mut Taffy {
        match self {
            Self::Layout { widget, .. } | Self::Measure { widget, .. } => widget,
        }
    }
}

impl Child {
    fn widget(&self) -> Option<&WidgetPod<dyn Widget>> {
        if let Child::Widget { widget, .. } = self {
            Some(widget)
        } else {
            None
        }
    }

    fn widget_mut(&mut self) -> Option<&mut WidgetPod<dyn Widget>> {
        if let Child::Widget { widget, .. } = self {
            Some(widget)
        } else {
            None
        }
    }

    fn style(&self) -> &taffy::Style {
        match self {
            Child::Widget { style, .. } | Child::Spacer { style, .. } => style,
        }
    }

    fn cache(&self) -> &taffy::Cache {
        match self {
            Child::Widget { cache, .. } | Child::Spacer { cache, .. } => cache,
        }
    }

    fn cache_mut(&mut self) -> &mut taffy::Cache {
        match self {
            Child::Widget { cache, .. } | Child::Spacer { cache, .. } => cache,
        }
    }
}

impl UsesProperty<Gap> for Taffy {}

// --- MARK: IMPL WIDGET
impl Widget for Taffy {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for widget in self.children.iter_mut().filter_map(Child::widget_mut) {
            ctx.register_child(widget);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Gap::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        if self.children.is_empty() {
            return 0.;
        }

        self.style.gap = taffy::style_helpers::FromLength::from_length(
            props.get::<Gap>(ctx.property_cache()).gap.get() as f32,
        );

        let input =
            convert::to_taffy_measure_constraints(ctx.context_size(), axis, len_req, cross_length);
        let node_id = taffy::NodeId::from(usize::MAX);

        let display_mode = self.style.display;

        // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
        let mut taffy_ctx = TaffyCtx::new_measure(self, ctx);
        let output = match display_mode {
            taffy::Display::Flex => taffy::compute_flexbox_layout(&mut taffy_ctx, node_id, input),
            taffy::Display::Grid => taffy::compute_grid_layout(&mut taffy_ctx, node_id, input),
            _ => unreachable!("the TaffyLayout widget only supports flex and grid display modes"),
        };

        output.size.get_abs(convert::to_taffy_axis(axis)) as _
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        if self.children.is_empty() {
            return;
        }

        self.style.gap = taffy::style_helpers::FromLength::from_length(
            props.get::<Gap>(ctx.property_cache()).gap.get() as f32,
        );

        let input = convert::to_taffy_layout_constraints(size);
        let node_id = taffy::NodeId::from(usize::MAX);

        let display_mode = self.style.display;

        // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
        let mut taffy_ctx = TaffyCtx::new_layout(self, ctx);
        match display_mode {
            taffy::Display::Flex => taffy::compute_flexbox_layout(&mut taffy_ctx, node_id, input),
            taffy::Display::Grid => taffy::compute_grid_layout(&mut taffy_ctx, node_id, input),
            _ => unreachable!("the TaffyLayout widget only supports flex and grid display modes"),
        };
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
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
            .filter_map(Child::widget)
            .map(WidgetPod::id)
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Taffy", id = id.trace())
    }
}

// --- MARK: IMPL TAFFYCTX
impl<'w, 'a> taffy::TraversePartialTree for TaffyCtx<'w, 'a> {
    type ChildIter<'c>
        = ChildIter
    where
        Self: 'c;

    fn child_ids(&self, _parent_node_id: taffy::NodeId) -> Self::ChildIter<'_> {
        ChildIter(0..self.widget().children.len())
    }

    fn child_count(&self, _parent_node_id: taffy::NodeId) -> usize {
        self.widget().children.len()
    }

    fn get_child_id(&self, _parent_node_id: taffy::NodeId, child_index: usize) -> taffy::NodeId {
        taffy::NodeId::from(child_index)
    }
}

impl<'w, 'a> taffy::LayoutPartialTree for TaffyCtx<'w, 'a> {
    type CoreContainerStyle<'b>
        = &'b taffy::Style
    where
        Self: 'b;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: taffy::NodeId) -> Self::CoreContainerStyle<'_> {
        let node_id = usize::from(node_id);
        if node_id == usize::MAX {
            &self.widget().style
        } else {
            &self.widget().children[node_id].style()
        }
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        if let Self::Layout { widget, ctx } = self
            && let Some(widget) = widget.children[usize::from(node_id)].widget_mut()
        {
            ctx.place_child(
                widget,
                (layout.location.x as f64, layout.location.y as f64).into(),
            )
        }
    }

    fn compute_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        inputs: taffy::LayoutInput,
    ) -> taffy::LayoutOutput {
        let idx = usize::from(node_id);
        match (self, inputs.run_mode) {
            (_, taffy::RunMode::PerformHiddenLayout) => {
                // TODO: set size of widget to zero
                taffy::LayoutOutput::HIDDEN
            }
            (Self::Layout { widget, ctx }, taffy::RunMode::PerformLayout)
                if let Some(size) = convert::to_size(inputs.known_dimensions) =>
            {
                if let Some(widget) = widget.children[idx].widget_mut() {
                    ctx.run_layout(widget, size);
                }
                let taffy_size = taffy::Size {
                    width: size.width as f32,
                    height: size.height as f32,
                };
                taffy::LayoutOutput::from_outer_size(taffy_size)
            }
            (Self::Layout { .. }, taffy::RunMode::PerformLayout) => {
                panic!("layout should have known size")
            }
            (Self::Layout { widget, ctx }, taffy::RunMode::ComputeSize) => {
                let size = if let Some(widget) = widget.children[idx].widget_mut() {
                    let (auto_size, context_size) = convert::to_measure_size_params(&inputs);
                    let size = ctx.compute_size(widget, auto_size, context_size);
                    taffy::Size {
                        width: size.width as f32,
                        height: size.height as f32,
                    }
                } else {
                    inputs
                        .available_space
                        .into_options()
                        .unwrap_or(taffy::Size::ZERO)
                };
                taffy::LayoutOutput::from_outer_size(size)
            }
            (Self::Measure { widget, ctx }, taffy::RunMode::ComputeSize) => {
                let size = if let Some(widget) = widget.children[idx].widget_mut() {
                    let (auto_length, context_size, axis, cross_length) =
                        convert::to_measure_len_params(&inputs);
                    let axis_size =
                        ctx.compute_length(widget, auto_length, context_size, axis, cross_length);
                    match inputs.axis {
                        taffy::RequestedAxis::Horizontal => taffy::Size {
                            width: axis_size as f32,
                            height: 0.,
                        },
                        taffy::RequestedAxis::Vertical => taffy::Size {
                            width: 0.,
                            height: axis_size as f32,
                        },
                        taffy::RequestedAxis::Both => unreachable!(),
                    }
                } else {
                    inputs
                        .available_space
                        .into_options()
                        .unwrap_or(taffy::Size::ZERO)
                };
                taffy::LayoutOutput::from_outer_size(size)
            }
            (Self::Measure { .. }, _) => panic!("measure run requires MeasureCtx"),
        }
    }
}

impl<'w, 'a> taffy::CacheTree for TaffyCtx<'w, 'a> {
    fn cache_get(
        &self,
        node_id: taffy::NodeId,
        input: &taffy::LayoutInput,
    ) -> Option<taffy::LayoutOutput> {
        match usize::from(node_id) {
            usize::MAX => self.widget().cache.get(input),
            idx => self.widget().children[idx].cache().get(input),
        }
    }

    fn cache_store(
        &mut self,
        node_id: taffy::NodeId,
        input: &taffy::LayoutInput,
        layout_output: taffy::LayoutOutput,
    ) {
        match usize::from(node_id) {
            usize::MAX => self.widget_mut().cache.store(input, layout_output),
            idx => self.widget_mut().children[idx]
                .cache_mut()
                .store(input, layout_output),
        }
    }

    fn cache_clear(&mut self, node_id: taffy::NodeId) {
        match usize::from(node_id) {
            usize::MAX => self.widget_mut().cache.clear(),
            idx => self.widget_mut().children[idx].cache_mut().clear(),
        };
    }
}

impl<'w, 'a> taffy::LayoutFlexboxContainer for TaffyCtx<'w, 'a> {
    type FlexboxContainerStyle<'b>
        = &'b taffy::Style
    where
        Self: 'b;

    type FlexboxItemStyle<'b>
        = &'b taffy::Style
    where
        Self: 'b;

    fn get_flexbox_container_style(
        &self,
        node_id: taffy::NodeId,
    ) -> Self::FlexboxContainerStyle<'_> {
        match usize::from(node_id) {
            usize::MAX => &self.widget().style,
            idx => &self.widget().children[idx].style(),
        }
    }

    fn get_flexbox_child_style(&self, node_id: taffy::NodeId) -> Self::FlexboxItemStyle<'_> {
        match usize::from(node_id) {
            usize::MAX => &self.widget().style,
            idx => &self.widget().children[idx].style(),
        }
    }
}

impl<'w, 'a> taffy::LayoutGridContainer for TaffyCtx<'w, 'a> {
    type GridContainerStyle<'b>
        = &'b taffy::Style
    where
        Self: 'b;

    type GridItemStyle<'b>
        = &'b taffy::Style
    where
        Self: 'b;

    fn get_grid_container_style(&self, node_id: taffy::NodeId) -> Self::GridContainerStyle<'_> {
        match usize::from(node_id) {
            usize::MAX => &self.widget().style,
            idx => &self.widget().children[idx].style(),
        }
    }

    fn get_grid_child_style(&self, node_id: taffy::NodeId) -> Self::GridItemStyle<'_> {
        match usize::from(node_id) {
            usize::MAX => &self.widget().style,
            idx => &self.widget().children[idx].style(),
        }
    }
}

// --- MARK: CONVERSIONS
/// Type conversions between Masonry types and their Taffy equivalents
mod convert {
    use super::*;
    use crate::layout::{LayoutSize, LenDef, SizeDef};

    pub(super) fn to_taffy_req_axis(axis: Axis) -> taffy::RequestedAxis {
        match axis {
            Axis::Horizontal => taffy::RequestedAxis::Horizontal,
            Axis::Vertical => taffy::RequestedAxis::Vertical,
        }
    }

    pub(super) fn to_taffy_axis(axis: Axis) -> taffy::AbsoluteAxis {
        match axis {
            Axis::Horizontal => taffy::AbsoluteAxis::Horizontal,
            Axis::Vertical => taffy::AbsoluteAxis::Vertical,
        }
    }

    pub(super) fn from_taffy_axis(axis: taffy::RequestedAxis) -> Axis {
        match axis {
            taffy::RequestedAxis::Horizontal => Axis::Horizontal,
            taffy::RequestedAxis::Vertical => Axis::Vertical,
            // Taffy only uses "both" axis when run mode is PerformLayout. So as long as we only call this function
            // when run mode is ComputeSize (which is the only time we care about axes) then this is unreachable.
            taffy::RequestedAxis::Both => unreachable!(),
        }
    }

    pub(super) fn to_taffy_layout_constraints(size: Size) -> taffy::LayoutInput {
        let available_space = taffy::Size {
            width: taffy::AvailableSpace::Definite(size.width as _),
            height: taffy::AvailableSpace::Definite(size.height as _),
        };

        let known_dimensions = taffy::Size::new(size.width as _, size.height as _);

        taffy::LayoutInput {
            known_dimensions,
            parent_size: taffy::Size::NONE,
            available_space,
            axis: taffy::RequestedAxis::Both,
            run_mode: taffy::RunMode::PerformLayout,
            sizing_mode: taffy::SizingMode::InherentSize,
            vertical_margins_are_collapsible: taffy::Line::FALSE,
        }
    }

    pub(super) fn to_taffy_measure_constraints(
        context_size: LayoutSize,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> taffy::LayoutInput {
        let available_len = match len_req {
            LenReq::FitContent(s) => taffy::AvailableSpace::Definite(s as _),
            LenReq::MinContent => taffy::AvailableSpace::MinContent,
            LenReq::MaxContent => taffy::AvailableSpace::MaxContent,
        };
        let available_space = match axis {
            Axis::Horizontal => taffy::Size {
                width: available_len,
                height: taffy::AvailableSpace::MaxContent,
            },
            Axis::Vertical => taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: available_len,
            },
        };

        let parent_size = taffy::Size {
            width: context_size.length(Axis::Horizontal).map(|l| l as _),
            height: context_size.length(Axis::Vertical).map(|l| l as _),
        };

        let cross_length = cross_length.map(|l| l as _);
        let known_dimensions = match axis {
            Axis::Horizontal => taffy::Size {
                width: None,
                height: cross_length,
            },
            Axis::Vertical => taffy::Size {
                width: cross_length,
                height: None,
            },
        };

        taffy::LayoutInput {
            known_dimensions,
            parent_size,
            available_space,
            axis: to_taffy_req_axis(axis),
            run_mode: taffy::RunMode::ComputeSize,
            sizing_mode: taffy::SizingMode::InherentSize,
            vertical_margins_are_collapsible: taffy::Line::FALSE,
        }
    }

    pub(super) fn to_size(known_dimensions: taffy::Size<Option<f32>>) -> Option<Size> {
        let known_dimensions = known_dimensions.map(|l| l.map(f64::from));
        if let Some(width) = known_dimensions.width
            && let Some(height) = known_dimensions.height
        {
            Some((width, height).into())
        } else {
            None
        }
    }

    pub(super) fn to_len_def(space: taffy::AvailableSpace) -> LenDef {
        match space {
            taffy::AvailableSpace::Definite(s) => LenDef::FitContent(s as _),
            taffy::AvailableSpace::MinContent => LenDef::MinContent,
            taffy::AvailableSpace::MaxContent => LenDef::MaxContent,
        }
    }

    pub(super) fn to_measure_len_params(
        input: &taffy::LayoutInput,
    ) -> (LenDef, LayoutSize, Axis, Option<f64>) {
        let axis = from_taffy_axis(input.axis);

        let auto_length = match axis {
            Axis::Horizontal => to_len_def(input.available_space.width),
            Axis::Vertical => to_len_def(input.available_space.height),
        };

        let context_length = match axis {
            Axis::Horizontal => input.parent_size.width,
            Axis::Vertical => input.parent_size.height,
        };
        let context_size = LayoutSize::maybe(axis, context_length.map(f64::from));

        let cross_length = match axis {
            Axis::Horizontal => input.known_dimensions.height.map(f64::from),
            Axis::Vertical => input.known_dimensions.width.map(f64::from),
        };

        (auto_length, context_size, axis, cross_length)
    }

    pub(super) fn to_measure_size_params(input: &taffy::LayoutInput) -> (SizeDef, LayoutSize) {
        let auto_size = SizeDef::new(
            to_len_def(input.available_space.width),
            to_len_def(input.available_space.height),
        );

        let context_size = to_size(input.parent_size).map_or(LayoutSize::NONE, Into::into);

        (auto_size, context_size)
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use taffy::style_helpers::*;

    use crate::properties::{Background, Dimensions};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::{layout::AsUnit, palette, theme::test_property_set, widgets::Label};

    use super::*;

    #[test]
    fn test_grid() {
        let widget = NewWidget::new(
            Taffy::new(taffy::Style {
                display: taffy::Display::Grid,
                grid_template_columns: vec![
                    taffy::GridTemplateComponent::MIN_CONTENT,
                    taffy::GridTemplateComponent::MAX_CONTENT,
                    taffy::GridTemplateComponent::from_fr(1.),
                ],
                grid_template_rows: vec![
                    taffy::GridTemplateComponent::AUTO,
                    taffy::GridTemplateComponent::from_length(20.),
                    taffy::GridTemplateComponent::from_percent(0.3),
                    taffy::GridTemplateComponent::from_fr(2.),
                    taffy::GridTemplateComponent::from_fr(1.),
                ],
                ..taffy::Style::DEFAULT
            })
            .with(
                Label::new("Min Content")
                    .prepare()
                    .with_props(Background::Color(palette::css::CHOCOLATE.with_alpha(0.5))),
                taffy::Style::DEFAULT,
            )
            .with(
                Label::new("Max Content")
                    .prepare()
                    .with_props(Background::Color(palette::css::OLIVE.with_alpha(0.5))),
                taffy::Style::DEFAULT,
            )
            .with(
                Label::new("1×3")
                    .prepare()
                    .with_props(Background::Color(palette::css::ORANGE_RED.with_alpha(0.5))),
                taffy::Style {
                    grid_row: taffy::Line {
                        start: taffy::GridPlacement::Line(1.into()),
                        end: taffy::GridPlacement::Span(3),
                    },
                    grid_column: taffy::Line::from_line_index(3),
                    ..taffy::Style::DEFAULT
                },
            )
            .with(
                Label::new("20px")
                    .prepare()
                    .with_props(Background::Color(palette::css::MAGENTA.with_alpha(0.5))),
                taffy::Style {
                    grid_column: taffy::Line::from_line_index(2),
                    ..taffy::Style::DEFAULT
                },
            )
            .with(
                Label::new("30%")
                    .prepare()
                    .with_props(Background::Color(palette::css::SEA_GREEN.with_alpha(0.5))),
                taffy::Style::DEFAULT,
            )
            .with(
                Label::new("2×2")
                    .prepare()
                    .with_props(Background::Color(palette::css::AQUAMARINE.with_alpha(0.5))),
                taffy::Style {
                    grid_row: taffy::Line {
                        start: taffy::GridPlacement::Line(3.into()),
                        end: taffy::GridPlacement::Span(2),
                    },
                    grid_column: taffy::Line {
                        start: taffy::GridPlacement::Line(2.into()),
                        end: taffy::GridPlacement::Span(2),
                    },
                    ..taffy::Style::DEFAULT
                },
            )
            .with(
                Label::new("2fr")
                    .prepare()
                    .with_props(Background::Color(palette::css::PURPLE.with_alpha(0.5))),
                taffy::Style::DEFAULT,
            )
            .with(
                Label::new("1fr")
                    .prepare()
                    .with_props(Background::Color(palette::css::GOLD.with_alpha(0.5))),
                taffy::Style {
                    grid_column: taffy::Line::from_line_index(3),
                    ..taffy::Style::DEFAULT
                },
            ),
        )
        .with_props(Gap::new(5.px()))
        .with_props(Dimensions::STRETCH);

        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (300, 400));
        assert_render_snapshot!(harness, "taffy_grid");
    }

    #[test]
    fn test_flex() {
        let widget = NewWidget::new(
            Taffy::new(taffy::Style {
                display: taffy::Display::Flex,
                ..taffy::Style::DEFAULT
            })
            .with(
                Label::new("Auto-sized label")
                    .prepare()
                    .with_props(Background::Color(palette::css::CHOCOLATE.with_alpha(0.5))),
                taffy::Style::DEFAULT,
            )
            .with_spacer(taffy::Style {
                flex_basis: taffy::Dimension::length(20.),
                ..taffy::Style::DEFAULT
            })
            .with(
                Label::new("Size specified label")
                    .prepare()
                    .with_props(Background::Color(palette::css::OLIVE.with_alpha(0.5)))
                    .with_props(Dimensions::width(150.px())),
                taffy::Style::DEFAULT,
            )
            .with(
                Label::new("Flexed label")
                    .prepare()
                    .with_props(Background::Color(palette::css::GOLD.with_alpha(0.5))),
                taffy::Style {
                    flex_grow: 2.,
                    ..taffy::Style::DEFAULT
                },
            )
            .with_spacer(taffy::Style {
                flex_grow: 1.,
                ..taffy::Style::DEFAULT
            }),
        )
        .with_props(Gap::new(5.px()))
        .with_props(Dimensions::STRETCH);

        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (500, 50));
        assert_render_snapshot!(harness, "taffy_flex");
    }

    #[test]
    fn test_flex_wrapping() {
        let mut widget = Taffy::new(taffy::Style {
            display: taffy::Display::Flex,
            flex_wrap: taffy::FlexWrap::Wrap,
            align_items: Some(taffy::AlignItems::Center),
            justify_items: Some(taffy::AlignItems::Center),
            ..taffy::Style::DEFAULT
        });
        for (i, color) in [
            palette::css::CHARTREUSE,
            palette::css::CHOCOLATE,
            palette::css::DODGER_BLUE,
            palette::css::GOLDENROD,
            palette::css::GREEN,
            palette::css::INDIAN_RED,
            palette::css::ORCHID,
            palette::css::PERU,
            palette::css::SIENNA,
        ]
        .into_iter()
        .enumerate()
        {
            widget = widget.with(
                Label::new((i + 1).to_string())
                    .prepare()
                    .with_props(Background::Color(color.with_alpha(0.5)))
                    .with_props(Dimensions::width(20.px())),
                taffy::Style {
                    flex_grow: 1.,
                    flex_shrink: 0.,
                    ..taffy::Style::DEFAULT
                },
            );
        }

        let widget = NewWidget::new(widget)
            .with_props(Gap::new(5.px()))
            .with_props(Dimensions::STRETCH);

        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (100, 60));
        assert_render_snapshot!(harness, "taffy_flex_wrapping");
    }
}
