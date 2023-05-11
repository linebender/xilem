// Copyright 2023 The Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use glazier::kurbo::Size;
use tracing::{trace, warn};

use super::*;

impl Widget for () {
    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut LifeCycleCx, _event: &LifeCycle) {}

    fn update(&mut self, _cx: &mut UpdateCx) {}

    fn layout(&mut self, _cx: &mut LayoutCx, _bc: &BoxConstraints) -> Size {
        Size::ZERO
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        cx.push_node(accesskit::NodeBuilder::new(accesskit::Role::Unknown));
    }

    fn paint(&mut self, _cx: &mut PaintCx, _builder: &mut vello::SceneBuilder) {}
}

pub struct Sizeable {
    pub(crate) child: Option<Pod>,
    pub(crate) width: Option<f64>,
    pub(crate) height: Option<f64>,
    pub(crate) old_size: Option<Size>,
}

impl Sizeable {
    fn child_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match self.width {
            Some(width) => {
                let w = width.clamp(bc.min().width, bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match self.height {
            Some(height) => {
                let h = height.clamp(bc.min().height, bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            (min_width, min_height).into(),
            (max_width, max_height).into(),
        )
    }
}

impl Widget for Sizeable {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if let Some(child) = &mut self.child {
            child.event(cx, event)
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        if let Some(child) = &mut self.child {
            child.lifecycle(cx, event)
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        if let Some(child) = &mut self.child {
            child.update(cx);
        } else {
            cx.request_layout();
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        // bc.constrain(if self.can_expand {
        //     (INFINITY, INFINITY)
        // } else {
        //     (self.space, self.space)
        // })

        bc.debug_check("SizedBox");

        let child_bc = self.child_constraints(bc);
        let size = match &mut self.child {
            Some(child) => child.layout(cx, &child_bc),
            None => bc.constrain((self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))),
        };

        trace!("Computed size: {}", size);
        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }

        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        if self.old_size != Some(size) {
            cx.request_paint();
            self.old_size = Some(size);
        }

        if let Some(child) = &mut self.child {
            child.state.flags |= PodFlags::REQUEST_PAINT;
        }

        size
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        if let Some(child) = &mut self.child {
            child.accessibility(cx);
        }

        if cx.is_requested() {
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::GenericContainer);
            if let Some(child) = &self.child {
                builder.set_children(vec![child.id().into()]);
            }
            cx.push_node(builder);
        }
        // if cx.is_requested() {
        //     if self.child.is_some() {
        //         cx.push_node(accesskit::NodeBuilder::new(accesskit::Role::Unknown));
        //     } else {
        //         let mut builder = accesskit::NodeBuilder::new(accesskit::Role::GenericContainer);
        //         builder.set_children(
        //             self.child
        //                 .as_ref()
        //                 .map(|pod| pod.id().into())
        //                 .into_iter()
        //                 .collect::<Vec<accesskit::NodeId>>(),
        //         );
        //         cx.push_node(builder);
        //     }
        // }
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut vello::SceneBuilder) {
        if let Some(child) = &mut self.child {
            child.paint(cx, builder)
        }
    }
}
