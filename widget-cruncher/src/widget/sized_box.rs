// Copyright 2019 The Druid Authors.
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

//! A widget with predefined size.

// TODO
// - get_child
// - get_child_mut
// - set_child
// - remove_child

use smallvec::{smallvec, SmallVec};
use std::f64::INFINITY;
use tracing::{trace, trace_span, warn, Span};

use crate::widget::prelude::*;
use crate::widget::{WidgetId, WidgetPod};
use crate::widget::{WidgetMut, WidgetRef};
use crate::Point;

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not
/// set, this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, SizedBox will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct SizedBox {
    child: Option<WidgetPod<Box<dyn Widget>>>,
    width: Option<f64>,
    height: Option<f64>,
}

impl SizedBox {
    /// Construct container with child, and both width and height not set.
    pub fn new(child: impl Widget) -> Self {
        Self {
            child: Some(WidgetPod::new(child).boxed()),
            width: None,
            height: None,
        }
    }

    /// Construct container with child, and both width and height not set.
    pub fn new_with_id(child: impl Widget, id: WidgetId) -> Self {
        Self {
            child: Some(WidgetPod::new_with_id(child, id).boxed()),
            width: None,
            height: None,
        }
    }

    /// Construct container without child, and both width and height not set.
    ///
    /// If the widget is unchanged, it will do nothing, which can be useful if you want to draw a
    /// widget some of the time (for example, it is used to implement
    /// [`Maybe`][crate::widget::Maybe]).
    #[doc(alias = "null")]
    pub fn empty() -> Self {
        Self {
            child: None,
            width: None,
            height: None,
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(INFINITY);
        self.height = Some(INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(INFINITY);
        self
    }
}

impl<'a, 'b> WidgetMut<'a, 'b, SizedBox> {
    pub fn set_child(&mut self, child: impl Widget) {
        self.widget.child = Some(WidgetPod::new(child).boxed());
        self.children_changed();
        self.request_layout();
    }

    pub fn remove_child(&mut self) {
        self.widget.child = None;
        self.children_changed();
        self.request_layout();
    }

    /// Set container's width.
    pub fn set_width(&mut self, width: f64) {
        self.widget.width = Some(width);
        self.request_layout();
    }

    /// Set container's height.
    pub fn set_height(&mut self, height: f64) {
        self.widget.height = Some(height);
        self.request_layout();
    }

    /// Set container's width.
    pub fn unset_width(&mut self) {
        self.widget.width = None;
        self.request_layout();
    }

    /// Set container's height.
    pub fn unset_height(&mut self) {
        self.widget.height = None;
        self.request_layout();
    }

    pub fn get_child_view(&mut self) -> Option<WidgetMut<'_, 'b, Box<dyn Widget>>> {
        let child = self.widget.child.as_mut()?;
        Some(WidgetMut {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: &mut child.state,
            widget: &mut child.inner,
        })
    }
}

impl SizedBox {
    fn child_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match self.width {
            Some(width) => {
                let w = width.max(bc.min().width).min(bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match self.height {
            Some(height) => {
                let h = height.max(bc.min().height).min(bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            Size::new(min_width, min_height),
            Size::new(max_width, max_height),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn width_and_height(&self) -> (Option<f64>, Option<f64>) {
        (self.width, self.height)
    }
}

impl Widget for SizedBox {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        ctx.init();
        if let Some(ref mut child) = self.child {
            child.on_event(ctx, event, env);
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        ctx.init();
        if let Some(ref mut child) = self.child {
            child.lifecycle(ctx, event, env)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();
        let child_bc = self.child_constraints(bc);

        let size;
        match self.child.as_mut() {
            Some(child) => {
                size = child.layout(ctx, &child_bc, env);
                child.set_origin(ctx, env, Point::ORIGIN);
                ctx.set_paint_insets(child.paint_insets());
            }
            None => size = bc.constrain((self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))),
        };

        trace!("Computed size: {}", size);

        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }
        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        ctx.init();
        if let Some(ref mut child) = self.child {
            child.paint(ctx, env);
        }
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        if let Some(child) = &self.child {
            smallvec![child.as_dyn()]
        } else {
            smallvec![]
        }
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("SizedBox")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::Harness;
    use crate::widget::Label;
    use insta::assert_debug_snapshot;

    #[test]
    fn expand() {
        let expand = SizedBox::new(Label::new("hello!")).expand();
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(400., 400.,));
    }

    #[test]
    fn no_width() {
        let expand = SizedBox::new(Label::new("hello!")).height(200.);
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc);
        assert_eq!(child_bc.min(), Size::new(0., 200.,));
        assert_eq!(child_bc.max(), Size::new(400., 200.,));
    }

    // TODO - screenshot tests aren't super useful since the box is transparent

    #[test]
    fn empty_box() {
        let widget = SizedBox::empty().width(40.0).height(40.0);

        let mut harness = Harness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "empty_box");
    }

    #[test]
    fn label_box_no_size() {
        let widget = SizedBox::new(Label::new("hello"));

        let mut harness = Harness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "label_box_no_size");
    }

    #[test]
    fn label_box_with_size() {
        let widget = SizedBox::new(Label::new("hello")).width(40.0).height(40.0);

        let mut harness = Harness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "label_box_no_size");
    }
}
