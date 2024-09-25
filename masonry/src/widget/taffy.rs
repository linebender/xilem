// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Line, Stroke};
use vello::Scene;
use taffy;
use taffy::AvailableSpace;

use crate::theme::get_debug_color;
use crate::widget::{WidgetMut};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

pub struct Taffy {
    children: Vec<Child>,
    style: taffy::Style,
}

struct Child {
    widget: WidgetPod<Box<dyn Widget>>,
    //style: taffy::Style,
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct TaffyParams {

}

/// Iterator over the widget's children. Used in the implementation of `taffy::TraversePartialTree`.
struct ChildIter(std::ops::Range<usize>);
impl Iterator for ChildIter {
    type Item = taffy::NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(taffy::NodeId::from)
    }
}

/// A wrapper view over the widget (`Taffy`) and the Masonry layout context (`LayoutCx`).
/// Implementing `taffy::PartialLayoutTree` for this wrapper (rather than implementing directing on
/// `TaffyLayout`) allows us to access the layout context during the layout process.
// TODO: Determine what information is still required for the updated partial tree.
struct TaffyLayoutCtx<'w, 'a> {
    /// A mutable reference to the widget
    widget: &'w mut Taffy,
    /// A mutable reference to the layout context
    ctx: &'w mut LayoutCtx<'a>,
}

impl<'w, 'a, 'b> TaffyLayoutCtx<'w, 'a> {
    /// Create a new `TaffyLayoutCtx`
    fn new(widget: &'w mut Taffy, ctx: &'w mut LayoutCtx<'a>) -> Self {
        TaffyLayoutCtx { widget, ctx }
    }
}

// --- MARK: IMPL TAFFY ---
impl Taffy {
    pub fn new(style: taffy::Style) -> Self {
        Taffy {
            children: Vec::new(),
            style,
        }
    }

    /// Builder-style variant of [`WidgetMut::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(self, child: impl Widget, params: TaffyParams) -> Self {
        self.with_child_pod(WidgetPod::new(Box::new(child)), params)
    }

    pub fn with_child_id(self, child: impl Widget, id: WidgetId, params: TaffyParams) -> Self {
        self.with_child_pod(WidgetPod::new_with_id(Box::new(child), id), params)
    }

    pub fn with_child_pod(
        mut self,
        widget: WidgetPod<Box<dyn Widget>>,
        params: TaffyParams,
    ) -> Self {
        let child = Child {
            widget,
        };
        self.children.push(child);
        self
    }
}

// --- MARK: IMPL CHILD ---
impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<Box<dyn Widget>>> {
        Some(&mut self.widget)
    }
    fn widget(&self) -> Option<&WidgetPod<Box<dyn Widget>>> {
        Some(&self.widget)
    }

    fn update_params(&mut self, params: TaffyParams) {
    }
}

fn new_taffy_child(params: TaffyParams, widget: WidgetPod<Box<dyn Widget>>) -> Child {
    Child {
        widget,
    }
}

// --- MARK: IMPL GRIDPARAMS ---
impl TaffyParams {
    pub fn new() -> TaffyParams {
        TaffyParams {}
    }
}

// --- MARK: WIDGETMUT---
impl<'a> WidgetMut<'a, Taffy> {
    /// Add a child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Taffy::with_child
    pub fn add_child(&mut self, child: impl Widget, params: TaffyParams) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new(Box::new(child));
        self.insert_child_pod(child_pod, params);
    }

    pub fn add_child_id(&mut self, child: impl Widget, id: WidgetId, params: TaffyParams) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new_with_id(Box::new(child), id);
        self.insert_child_pod(child_pod, params);
    }

    /// Add a child widget.
    pub fn insert_child_pod(&mut self, widget: WidgetPod<Box<dyn Widget>>, params: TaffyParams) {
        let child = new_taffy_child(params, widget);
        self.widget.children.push(child);
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn insert_taffy_child_at(
        &mut self,
        idx: usize,
        child: impl Widget,
        params: impl Into<TaffyParams>,
    ) {
        self.insert_taffy_child_pod(idx, WidgetPod::new(Box::new(child)), params);
    }

    pub fn insert_taffy_child_pod(
        &mut self,
        idx: usize,
        child: WidgetPod<Box<dyn Widget>>,
        params: impl Into<TaffyParams>,
    ) {
        let child = new_taffy_child(params.into(), child);
        self.widget.children.insert(idx, child);
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn child_mut(&mut self, idx: usize) -> Option<WidgetMut<'_, Box<dyn Widget>>> {
        let child = match self.widget.children[idx].widget_mut() {
            Some(widget) => widget,
            None => return None,
        };

        Some(self.ctx.get_mut(child))
    }

    /// Updates the taffy parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_taffy_params(&mut self, idx: usize, params: TaffyParams) {
        let child = &mut self.widget.children[idx];
        child.update_params(params);
        self.ctx.request_layout();
    }

    pub fn remove_child(&mut self, idx: usize) {
        let child = self.widget.children.remove(idx);
        self.ctx.remove_child(child.widget);
        self.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for Taffy {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.lifecycle(ctx, event);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        bc.debug_check("Taffy");

        let display_mode = self.style.display;
        let has_children = !self.children.is_empty();

        let inputs = convert::to_taffy_constraints(
            bc,
            taffy::RequestedAxis::Both,
            taffy::RunMode::PerformLayout,
            taffy::SizingMode::InherentSize,
        );
        let node_id = taffy::NodeId::from(usize::MAX);

        // TODO: Cache get

        // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
        let mut layout_ctx = TaffyLayoutCtx::new(self, ctx);
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
            (_, false) => {
                taffy::compute_leaf_layout(inputs, &self.style, |known_dimensions, available_space| {
                    // TODO: This is a fixed value until the measure function is done.
                    taffy::geometry::Size{
                        width: 1.0,
                        height: 1.0,
                    }
                })
            }
        };

        // TODO: Cache set

        let max = bc.max();
        Size {
            width: (output.size.width as f64).min(max.width),
            height: (output.size.height as f64).min(max.height),
        }


        /*let total_size = bc.max();
        for child in &mut self.children {
            let child_bc = bc;
            let _ = ctx.run_layout(&mut child.widget, &child_bc);
            ctx.place_child(
                &mut child.widget,
                Point::new(0.0, 0.0),
            );
        }*/
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // paint the baseline if we're debugging layout
        if ctx.debug_paint && ctx.widget_state.baseline_offset != 0.0 {
            let color = get_debug_color(ctx.widget_id().to_raw());
            let my_baseline = ctx.size().height - ctx.widget_state.baseline_offset;
            let line = Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _: &mut AccessCtx) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Taffy")
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
        //if node_id == usize::MAX {
        &self.widget.style
        // TODO: Per-child style
        /*} else {
            let child = &self.widget.children[node_id];
            match child {
                Some(child_widget) => &child_widget.style,
                None => {
                    static DEFAULT_STYLE: taffy::Style = taffy::Style::DEFAULT;
                    &DEFAULT_STYLE
                }
            }
        }*/
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        self.ctx.place_child(
            &mut self.widget.children[usize::from(node_id)].widget,
            Point {
                x: layout.location.x as f64,
                y: layout.location.y as f64,
            }
        )
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
                let size = self.ctx.run_layout(&mut child.widget, &box_constraints);
                let taffy_size = taffy::Size {
                    width: size.width as f32,
                    height: size.height as f32,
                };
                taffy::LayoutOutput::from_outer_size(taffy_size)
            }
            taffy::RunMode::ComputeSize => {
                let child = &mut self.widget.children[usize::from(node_id)];
                let axis_size = self.ctx.run_measure(&mut child.widget, &box_constraints, convert::from_taffy_axis(input.axis));
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

/// Type conversions between Masonry types and their Taffy equivalents
mod convert {
    use vello::kurbo::Size;
    use crate::BoxConstraints;
    use crate::widget::Axis;

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
        /// Converts Taffy's known_dimension and available_spaceinto a min box constraint
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

        /// Converts Taffy's known_dimension and available_spaceinto a min box constraint
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

