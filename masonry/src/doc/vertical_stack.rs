// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// This file is the template for the VerticalStack examples in docs.
// Because these examples each include chunklets of code, it's hard to make them compile.
// Instead, make sure this file compiles, and whenever you change this file, make sure to
// also mirror the change in the respective examples.

// TODO - Find some way to check that code chunks in docs
// are up to date with this file.

use crate as masonry;

// Note: The "// ---" lines separate blocks of code which are included together in
// a tutorial example. So for example, the first code block in the widget container tutorial
// imports Widget and WidgetPod, and then successive code blocks import more items.

use masonry::core::{Widget, WidgetPod};
// ---
use masonry::core::{LayoutCtx, MeasureCtx, PropertiesRef};
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, SizeDef};
// ---
use masonry::core::ComposeCtx;
// ---
use masonry::core::RegisterCtx;
// ---
use masonry::core::ChildrenIds;
// ---
use masonry::core::{NewWidget, WidgetMut};
// ---
use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, EventCtx, NoAction, PaintCtx, PointerEvent, PropertiesMut, TextEvent,
    Update, UpdateCtx,
};
use masonry::vello::Scene;

// ---

pub struct VerticalStack {
    children: Vec<WidgetPod<dyn Widget>>,
    gap: f64,
}

impl VerticalStack {
    pub fn new(gap: f64) -> Self {
        Self {
            children: Vec::new(),
            gap,
        }
    }
}

// ---

impl VerticalStack {
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.widget.children.push(child.to_pod());
        this.ctx.children_changed();
    }

    pub fn remove_child(this: &mut WidgetMut<'_, Self>, n: usize) {
        this.widget.children.remove(n);
        this.ctx.children_changed();
    }

    pub fn clear_children(this: &mut WidgetMut<'_, Self>) {
        this.widget.children.clear();
        this.ctx.children_changed();
    }
}

// ---

#[rustfmt::skip]
impl Widget for VerticalStack {
    type Action = NoAction;

    fn on_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &AccessEvent) {}

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _interval: u64) {}
    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    // ---

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let (len_req, min_result) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (len_req, 0.),
            LenReq::FitContent(space) => (LenReq::MinContent, space),
        };

        let auto_size = SizeDef::req(axis, len_req);
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);
        
        let mut length: f64 = 0.;
        for child in &mut self.children {
            let child_length = ctx.compute_length(child, auto_size, context_size, axis, cross_length);
            match axis {
                Axis::Horizontal => length = length.max(child_length),
                Axis::Vertical => length += child_length,
            }
        }

        if axis == Axis::Vertical {
            let gap_count = (self.children.len() - 1) as f64;
            length += gap_count * self.gap;
        }

        min_result.max(length)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &PropertiesRef<'_>,
        size: Size,
    ) {
        let gap_count = (self.children.len() - 1) as f64;
        let total_child_vertical_space = size.height - self.gap * gap_count;
        let child_vertical_space = total_child_vertical_space / self.children.len() as f64;

        let width_def = LenDef::FitContent(size.width);
        let height_def = LenDef::FitContent(child_vertical_space.max(0.));
        let auto_size = SizeDef::new(width_def, height_def);
        let context_size = size.into();

        let mut y_offset = 0.0;
        for child in &mut self.children {
            let child_size = ctx.compute_size(child, auto_size, context_size);
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(0.0, y_offset));

            y_offset += child_size.height + self.gap;
        }
    }

    // ---

    fn compose(&mut self, _ctx: &mut ComposeCtx<'_>) {}

    // ---

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

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

    // ---

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(child);
        }
    }

    // ---

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(|child| child.id()).collect()
    }
}
