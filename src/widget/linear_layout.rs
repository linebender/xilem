// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::geometry::Axis;
use crate::widget::{BoxConstraints, Event};
use vello::kurbo::Size;
use vello::Scene;

use super::{contexts::LifeCycleCx, EventCx, LayoutCx, LifeCycle, PaintCx, Pod, UpdateCx, Widget};

/// LinearLayout is a simple widget which does layout for a ViewSequence.
///
/// Each Element is positioned on the specified Axis starting at the beginning with the given spacing
///
/// This Widget is only temporary and is probably going to be replaced by something like Druid's Flex
/// widget.
pub struct LinearLayout {
    pub children: Vec<Pod>,
    pub spacing: f64,
    pub axis: Axis,
}

impl LinearLayout {
    pub fn new(children: Vec<Pod>, spacing: f64, axis: Axis) -> Self {
        LinearLayout {
            children,
            spacing,
            axis,
        }
    }
}

impl Widget for LinearLayout {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(cx, event);
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        for child in &mut self.children {
            child.update(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let child_bc = self.axis.with_major(*bc, 0.0..f64::INFINITY);
        let child_count = self.children.len();

        let mut major_used: f64 = 0.0;
        let mut max_minor: f64 = 0.0;

        for (index, child) in self.children.iter_mut().enumerate() {
            let size = child.layout(cx, &child_bc);
            child.set_origin(cx, self.axis.pack(major_used, 0.0));
            major_used += self.axis.major(size);
            if index < child_count - 1 {
                major_used += self.spacing;

                // println!("insert spacing {}", self.spacing);
            }
            max_minor = max_minor.max(self.axis.minor(size));
        }

        self.axis.pack(major_used, max_minor)
    }

    fn paint(&mut self, cx: &mut PaintCx, scene: &mut Scene) {
        for child in &mut self.children {
            // println!("paint child!");
            child.paint(cx, scene);
        }
    }
}
