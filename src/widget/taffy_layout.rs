// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use masonry::{
    declare_widget,
    widget::{Axis, WidgetRef},
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent,
    StatusChange, TextEvent, Widget, WidgetPod,
};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::{
    kurbo::{Affine, Point, Size},
    peniko::{Brush, Color, Fill},
    Scene,
};

/// Type conversions between Xilem types and their Taffy equivalents
mod convert {
    use crate::{widget::BoxConstraints, Axis};
    use vello::kurbo::Size;

    /// Convert a `xilem::Axis` to a `taffy::AbsoluteAxis`
    pub(super) fn to_taffy_axis(axis: Axis) -> taffy::AbsoluteAxis {
        match axis {
            Axis::Horizontal => taffy::AbsoluteAxis::Horizontal,
            Axis::Vertical => taffy::AbsoluteAxis::Vertical,
        }
    }

    /// Convert a `taffy::RequestedAxis` to a `xilem::Axis`
    pub(super) fn from_taffy_axis(axis: taffy::RequestedAxis) -> Axis {
        match axis {
            taffy::RequestedAxis::Horizontal => Axis::Horizontal,
            taffy::RequestedAxis::Vertical => Axis::Vertical,
            // Taffy only uses "both" axis when run mode is PerformLayout. So as long as we only call this function
            // when run mode is ComputeSize (which is the only time we care about axes) then this is unreachable.
            taffy::RequestedAxis::Both => unreachable!(),
        }
    }

    /// Convert `xilem::BoxConstraints` to `taffy::LayoutInput`.
    pub(super) fn to_taffy_constraints(
        bc: &BoxConstraints,
        axis: taffy::RequestedAxis,
        run_mode: taffy::RunMode,
        sizing_mode: taffy::SizingMode,
    ) -> taffy::LayoutInput {
        /// Convert min and max box constraints into a `taffy::AvailableSpace`
        fn to_available_space(min: f64, max: f64) -> taffy::AvailableSpace {
            if max.is_finite() {
                taffy::AvailableSpace::Definite(max as f32)
            } else if min.is_sign_negative() {
                taffy::AvailableSpace::MinContent
            } else {
                taffy::AvailableSpace::MaxContent
            }
        }

        let min = bc.min();
        let max = bc.max();

        taffy::LayoutInput {
            known_dimensions: taffy::Size {
                width: (min.width == max.width && min.width.is_finite())
                    .then_some(min.width as f32),
                height: (min.height == max.height && min.height.is_finite())
                    .then_some(min.height as f32),
            },
            parent_size: taffy::Size {
                width: max.width.is_finite().then_some(max.width as f32),
                height: max.height.is_finite().then_some(max.height as f32),
            },
            available_space: taffy::Size {
                width: to_available_space(min.width, max.width),
                height: to_available_space(min.height, max.height),
            },
            axis,
            run_mode,
            sizing_mode,
            vertical_margins_are_collapsible: taffy::Line::FALSE,
        }
    }

    /// Convert`taffy::LayoutInput` to `xilem::BoxConstraints`
    pub(super) fn to_box_constraints(input: &taffy::LayoutInput) -> BoxConstraints {
        /// Converts Taffy's `known_dimension` and `available_space` into a min box constraint
        fn to_min_constraint(
            known_dimension: Option<f32>,
            available_space: taffy::AvailableSpace,
        ) -> f64 {
            known_dimension.unwrap_or(match available_space {
                taffy::AvailableSpace::Definite(_) => 0.0,
                taffy::AvailableSpace::MaxContent => 0.0,
                taffy::AvailableSpace::MinContent => -0.0,
            }) as f64
        }

        /// Converts Taffy's `known_dimension` and `available_space` into a min box constraint
        fn to_max_constraint(
            known_dimension: Option<f32>,
            available_space: taffy::AvailableSpace,
        ) -> f64 {
            known_dimension.unwrap_or(match available_space {
                taffy::AvailableSpace::Definite(val) => val,
                taffy::AvailableSpace::MaxContent => f32::INFINITY,
                taffy::AvailableSpace::MinContent => f32::INFINITY,
            }) as f64
        }

        BoxConstraints::new(
            Size {
                width: to_min_constraint(input.known_dimensions.width, input.available_space.width),
                height: to_min_constraint(
                    input.known_dimensions.height,
                    input.available_space.height,
                ),
            },
            Size {
                width: to_max_constraint(input.known_dimensions.width, input.available_space.width),
                height: to_max_constraint(
                    input.known_dimensions.height,
                    input.available_space.height,
                ),
            },
        )
    }
}

/// `TaffyLayout` is a container view which does layout for the specified `ViewSequence`.
///
/// Children are positioned according to the Block, Flexbox or CSS Grid algorithm, depending
/// on the display style set. If the children are themselves instances of `TaffyLayout`, then
/// they can set styles to control how they placed, sized, and aligned.
pub struct TaffyLayout {
    pub children: Vec<WidgetPod<Box<dyn Widget>>>,
    pub cache: taffy::Cache,
    pub style: taffy::Style,
    pub background_color: Option<Color>,
}

declare_widget!(TaffyLayoutMut, TaffyLayout);

impl TaffyLayout {
    pub fn new(
        children: Vec<WidgetPod<Box<dyn Widget>>>,
        style: taffy::Style,
        background_color: Option<Color>,
    ) -> Self {
        TaffyLayout {
            children,
            cache: taffy::Cache::new(),
            style,
            background_color,
        }
    }
}

/// Iterator over the widget's children. Used in the implementation of `taffy::TraversePartialTree`.
struct ChildIter(std::ops::Range<usize>);
impl Iterator for ChildIter {
    type Item = taffy::NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(taffy::NodeId::from)
    }
}

/// A this wrapper view over the widget (`TaffyLayout`) and the Xilem layout context (`LayoutCtx`).
/// Implementing `taffy::TraversePartialTree` and `taffy::LayoutPartialTree` for this wrapper (rather
/// than directly on `TaffyLayout`) allows us to access the layout context during the layout process
struct TaffyLayoutCtx<'w, 'a> {
    /// A mutable reference to the widget
    widget: &'w mut TaffyLayout,
    /// A mutable reference to the layout context
    cx: &'w mut LayoutCtx<'a>,
}

impl<'w, 'a> TaffyLayoutCtx<'w, 'a> {
    /// Create a new `TaffyLayoutCtx`
    fn new(widget: &'w mut TaffyLayout, cx: &'w mut LayoutCtx<'a>) -> Self {
        TaffyLayoutCtx { widget, cx }
    }
}

impl<'w, 'a> taffy::TraversePartialTree for TaffyLayoutCtx<'w, 'a> {
    type ChildIter<'c> = ChildIter where Self: 'c;

    fn child_ids(&self, _parent_node_id: taffy::NodeId) -> Self::ChildIter<'_> {
        ChildIter(0..self.widget.children.len())
    }

    fn child_count(&self, _parent_node_id: taffy::NodeId) -> usize {
        self.widget.children.len()
    }

    fn get_child_id(&self, _parent_node_id: taffy::NodeId, child_index: usize) -> taffy::NodeId {
        taffy::NodeId::from(child_index)
    }
}

impl<'w, 'a> taffy::LayoutPartialTree for TaffyLayoutCtx<'w, 'a> {
    fn get_style(&self, node_id: taffy::NodeId) -> &taffy::Style {
        let node_id = usize::from(node_id);
        if node_id == usize::MAX {
            &self.widget.style
        } else {
            let child = &self.widget.children[node_id];
            match child.downcast_ref::<TaffyLayout>() {
                Some(child_widget) => &child_widget.style,
                None => {
                    static DEFAULT_STYLE: taffy::Style = taffy::Style::DEFAULT;
                    &DEFAULT_STYLE
                }
            }
        }
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        self.widget.children[usize::from(node_id)].set_origin(
            self.cx,
            Point {
                x: layout.location.x as f64,
                y: layout.location.y as f64,
            },
        );
    }

    fn get_cache_mut(&mut self, _node_id: taffy::NodeId) -> &mut taffy::Cache {
        // We are implementing our own caching strategy rather than using the `taffy::compute_cached_layout` method
        // so this method will never be called
        unimplemented!()
    }

    fn compute_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        input: taffy::LayoutInput,
    ) -> taffy::LayoutOutput {
        let box_constraints: BoxConstraints = convert::to_box_constraints(&input);
        match input.run_mode {
            taffy::RunMode::PerformLayout => {
                let child = &mut self.widget.children[usize::from(node_id)];
                let size = child.layout(self.cx, &box_constraints);
                let taffy_size = taffy::Size {
                    width: size.width as f32,
                    height: size.height as f32,
                };
                taffy::LayoutOutput::from_outer_size(taffy_size)
            }
            taffy::RunMode::ComputeSize => {
                let axis_size = self.widget.children[usize::from(node_id)].compute_max_intrinsic(
                    convert::from_taffy_axis(input.axis),
                    self.cx,
                    &box_constraints,
                );
                let taffy_size = match input.axis {
                    taffy::RequestedAxis::Horizontal => taffy::Size {
                        width: axis_size as f32,
                        height: 0.0,
                    },
                    taffy::RequestedAxis::Vertical => taffy::Size {
                        width: 0.0,
                        height: axis_size as f32,
                    },
                    taffy::RequestedAxis::Both => unreachable!(),
                };
                taffy::LayoutOutput::from_outer_size(taffy_size)
            }
            taffy::RunMode::PerformHiddenLayout => {
                // TODO: set size of widget to zero
                taffy::LayoutOutput::HIDDEN
            }
        }
    }
}

impl Widget for TaffyLayout {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        for child in &mut self.children {
            child.on_pointer_event(ctx, event);
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        for child in &mut self.children {
            child.on_text_event(ctx, event);
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(ctx, event);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let display_mode = self.style.display;
        let has_children = !self.children.is_empty();

        let inputs = convert::to_taffy_constraints(
            bc,
            taffy::RequestedAxis::Both,
            taffy::RunMode::PerformLayout,
            taffy::SizingMode::InherentSize,
        );
        let node_id = taffy::NodeId::from(usize::MAX);

        // Invalidate cache on child layout request.
        if self.children.iter().any(|child| child.layout_requested()) {
            self.cache.clear();
        }

        // Check for cached layout. And return it if found.
        if let Some(cached_output) = self.cache.get(
            inputs.known_dimensions,
            inputs.available_space,
            taffy::RunMode::PerformLayout,
        ) {
            let max = bc.max();
            return Size {
                width: (cached_output.size.width as f64).min(max.width),
                height: (cached_output.size.height as f64).min(max.height),
            };
        }

        // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
        let mut layout_ctx = TaffyLayoutCtx::new(self, cx);
        let output = match (display_mode, has_children) {
            (taffy::Display::None, _) => taffy::compute_hidden_layout(&mut layout_ctx, node_id),
            (taffy::Display::Block, true) => {
                taffy::compute_block_layout(&mut layout_ctx, node_id, inputs)
            }
            (taffy::Display::Flex, true) => {
                taffy::compute_flexbox_layout(&mut layout_ctx, node_id, inputs)
            }
            (taffy::Display::Grid, true) => {
                taffy::compute_grid_layout(&mut layout_ctx, node_id, inputs)
            }
            (_, false) => taffy::compute_leaf_layout(inputs, &self.style, |_, _| taffy::Size::ZERO),
        };

        // Save output to cache
        self.cache.store(
            inputs.known_dimensions,
            inputs.available_space,
            taffy::RunMode::PerformLayout,
            output,
        );

        cx.request_paint();

        let max = bc.max();
        Size {
            width: (output.size.width as f64).min(max.width),
            height: (output.size.height as f64).min(max.height),
        }
    }

    // TODO
    #[cfg(FALSE)]
    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        cx: &mut LayoutCtx,
        bc: &BoxConstraints,
    ) -> f64 {
        let display_mode = self.style.display;
        let has_children = !self.children.is_empty();

        let node_id = taffy::NodeId::from(usize::MAX);
        let taffy_axis = convert::to_taffy_axis(axis);
        let inputs = convert::to_taffy_constraints(
            bc,
            taffy_axis.into(),
            taffy::RunMode::ComputeSize,
            taffy::SizingMode::InherentSize, // TODO: Support SizingMode::ContentSize
        );

        // Check for cached size. And return it if found.
        if let Some(cached_output) = self.cache.get(
            inputs.known_dimensions,
            inputs.available_space,
            taffy::RunMode::ComputeSize,
        ) {
            return cached_output.size.get_abs(taffy_axis) as f64;
        }

        // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
        let mut layout_ctx = TaffyLayoutCtx::new(self, cx);
        let output = match (display_mode, has_children) {
            (taffy::Display::None, _) => taffy::compute_hidden_layout(&mut layout_ctx, node_id),
            (taffy::Display::Block, true) => {
                taffy::compute_block_layout(&mut layout_ctx, node_id, inputs)
            }
            (taffy::Display::Flex, true) => {
                taffy::compute_flexbox_layout(&mut layout_ctx, node_id, inputs)
            }
            (taffy::Display::Grid, true) => {
                taffy::compute_grid_layout(&mut layout_ctx, node_id, inputs)
            }
            (_, false) => taffy::compute_leaf_layout(inputs, &self.style, |_, _| taffy::Size::ZERO),
        };

        // Save output to cache
        self.cache.store(
            inputs.known_dimensions,
            inputs.available_space,
            taffy::RunMode::ComputeSize,
            output,
        );

        output.size.get_abs(taffy_axis) as f64
    }

    fn paint(&mut self, cx: &mut PaintCtx, scene: &mut Scene) {
        if let Some(color) = self.background_color {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(color),
                None,
                &cx.size().to_rect(),
            );
        }
        for child in &mut self.children {
            child.paint(cx, scene);
        }
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        self.children
            .iter()
            .map(|widget_pod| widget_pod.as_dyn())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Flex")
    }
}
