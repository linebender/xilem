// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::Role;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Line, Stroke};
use vello::Scene;

use crate::theme::get_debug_color;
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

pub struct Grid {
    children: Vec<Child>,
    grid_width: i32,
    grid_height: i32,
    grid_spacing: f64,
    //old_bc: BoxConstraints,
    //needs_layout: bool,
}

// --- MARK: IMPL GRID ---
impl Grid {
    pub fn with_dimensions(width: i32, height: i32) -> Self {
        Grid {
            children: Vec::new(),
            grid_width: width,
            grid_height: height,
            grid_spacing: 0.0,
            //old_bc: BoxConstraints::new(Size::ZERO, Size::ZERO),
            //needs_layout: true,
        }
    }

    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.grid_spacing = spacing;
        self
    }

    /// Builder-style variant of [`WidgetMut::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(self, child: impl Widget, params: GridParams) -> Self {
        self.with_child_pod(WidgetPod::new(Box::new(child)), params)
    }

    pub fn with_child_id(self, child: impl Widget, id: WidgetId, params: GridParams) -> Self {
        self.with_child_pod(WidgetPod::new_with_id(Box::new(child), id), params)
    }

    pub fn with_child_pod(mut self, widget: WidgetPod<Box<dyn Widget>>, params: GridParams) -> Self {
        let child = Child {
            widget,
            x: params.x,
            y: params.y,
            width: params.width,
            height: params.height,
        };
        self.children.push(child);
        self
    }
}

// --- MARK: WIDGETMUT---
impl<'a> WidgetMut<'a, Grid> {
    /// Add a child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Grid::with_child
    pub fn add_child(&mut self, child: impl Widget, params: GridParams) {
        let child_pod: WidgetPod<Box<dyn Widget>>  = WidgetPod::new(Box::new(child));
        self.insert_child_pod(child_pod, params);
    }

    pub fn add_child_id(&mut self, child: impl Widget, id: WidgetId, params: GridParams) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new_with_id(Box::new(child), id);
        self.insert_child_pod(child_pod, params);
    }

    /// Add a child widget.
    pub fn insert_child_pod(&mut self, widget: WidgetPod<Box<dyn Widget>>, params: GridParams) {
        let child = Child {
            widget,
            x: params.x,
            y: params.y,
            width: params.width,
            height: params.height,
        };
        self.widget.children.push(child);
        self.ctx.children_changed();
        self.mark_needs_layout();
    }

    pub fn insert_grid_child(
        &mut self,
        idx: usize,
        child: impl Widget,
        params: impl Into<GridParams>,
    ) {
        self.insert_grid_child_pod(idx, WidgetPod::new(Box::new(child)), params);
    }

    pub fn insert_grid_child_pod(
        &mut self,
        idx: usize,
        child: WidgetPod<Box<dyn Widget>>,
        params: impl Into<GridParams>,
    ) {
        let child = new_grid_child(params.into(), child);
        self.widget.children.insert(idx, child);
        self.ctx.children_changed();

        self.mark_needs_layout();
    }

    pub fn set_spacing(&mut self, spacing: f64) {
        self.widget.grid_spacing = spacing;
        self.mark_needs_layout();
    }

    pub fn set_width(&mut self, width: i32) {
        self.widget.grid_width = width;
        self.mark_needs_layout();
    }

    pub fn set_height(&mut self, height: i32) {
        self.widget.grid_height = height;
        self.mark_needs_layout();
    }

    /// Used to force a re-layout.
    fn mark_needs_layout(&mut self) {
        //self.widget.needs_layout = true;
        self.ctx.request_layout();
    }

    pub fn child_mut(&mut self, idx: usize) -> Option<WidgetMut<'_, Box<dyn Widget>>>{
        let child = match self.widget.children[idx].widget_mut() {
            Some(widget) => widget,
            None => return None,
        };

        Some(self.ctx.get_mut(child))
    }

    /// Updates the grid parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_grid_params(&mut self, idx: usize, params: GridParams) {
        let child = &mut self.widget.children[idx];
        child.update_params(params);
        self.mark_needs_layout();
    }

    pub fn remove_child(&mut self, idx: usize) {
        let child = self.widget.children.remove(idx);
        self.ctx.remove_child(child.widget);
        self.mark_needs_layout();
    }
}

fn new_grid_child(params: GridParams, widget: WidgetPod<Box<dyn Widget>>) -> Child {
    Child{
        widget,
        x: params.x,
        y: params.y,
        width: params.width,
        height: params.height,
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for Grid {
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
        bc.debug_check("Grid");
        /*let bc_changed = self.old_bc != *bc;
        if bc_changed {
            self.old_bc = *bc;
            if !self.needs_layout {
                self.needs_layout = true;
            }
        }*/
        let total_size = bc.max();
        let width_unit = (total_size.width + self.grid_spacing) / (self.grid_width as f64);
        let height_unit = (total_size.height + self.grid_spacing) / (self.grid_height as f64);
        for child in &mut self.children {
            /*if !self.needs_layout && !ctx.child_needs_layout(&child.widget) {
                ctx.mark_child_as_visited(&child.widget, true);
                continue; // TODO: This breaks it. This is an attempted optimization.
            }*/
            let cell_size = Size::new(
                child.width as f64 * width_unit - self.grid_spacing,
                child.height as f64 * height_unit - self.grid_spacing,
            );
            let child_bc = BoxConstraints::new(cell_size, cell_size);
            let _ = child.widget.layout(ctx, &child_bc);
            ctx.place_child(&mut child.widget, Point::new(child.x as f64 *width_unit, child.y as f64 * height_unit))
        }
        /*if self.needs_layout {
            self.needs_layout = false;
        }*/
        total_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // Just paint every child
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.paint(ctx, scene);
        }

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

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.accessibility(ctx);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Grid")
    }
}

struct Child {
    widget: WidgetPod<Box<dyn Widget>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<Box<dyn Widget>>> {
        Some(&mut self.widget)
    }
    fn widget(&self) -> Option<&WidgetPod<Box<dyn Widget>>> {
        Some(&self.widget)
    }

    fn update_params(&mut self, params: GridParams) {
        self.x = params.x;
        self.y = params.y;
        self.width = params.width;
        self.height = params.height;
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct GridParams {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl GridParams {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> GridParams {
        GridParams{
            x,
            y,
            width,
            height,
        }
    }
}