// Copyright 2020 The Druid Authors.
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

#![cfg(not(tarpaulin_include))]

use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::widget::prelude::*;
use crate::widget::widget_view::WidgetRef;
use crate::widget::widget_view::WidgetView;
use crate::widget::Axis;
use crate::WidgetPod;
use druid_shell::kurbo::Shape;
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, warn, Span};

/// Represents the size and position of a rectangular "viewport" into a larger area.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Viewport {
    /// The size of the area that we have a viewport into.
    pub content_size: Size,
    /// The origin of the view rectangle.
    pub view_origin: Point,
    /// The size of the view rectangle.
    pub view_size: Size,
}

impl Viewport {
    /// The view rectangle.
    pub fn view_rect(&self) -> Rect {
        Rect::from_origin_size(self.view_origin, self.view_size)
    }

    /// Tries to find a position for the view rectangle that is contained in the content rectangle.
    ///
    /// If the supplied origin is good, returns it; if it isn't, we try to return the nearest
    /// origin that would make the view rectangle contained in the content rectangle. (This will
    /// fail if the content is smaller than the view, and we return `0.0` in each dimension where
    /// the content is smaller.)
    pub fn clamp_view_origin(&self, origin: Point) -> Point {
        let x = origin
            .x
            .min(self.content_size.width - self.view_size.width)
            .max(0.0);
        let y = origin
            .y
            .min(self.content_size.height - self.view_size.height)
            .max(0.0);
        Point::new(x, y)
    }

    /// Changes the viewport offset by `delta`, while trying to keep the view rectangle inside the
    /// content rectangle.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn pan_by(&mut self, delta: Vec2) -> bool {
        self.pan_to(self.view_origin + delta)
    }

    /// Sets the viewport origin to `pos`, while trying to keep the view rectangle inside the
    /// content rectangle.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, origin: Point) -> bool {
        let new_origin = self.clamp_view_origin(origin);
        if (new_origin - self.view_origin).hypot2() > 1e-12 {
            self.view_origin = new_origin;
            true
        } else {
            false
        }
    }

    /// Pan the smallest distance that makes the target [`Rect`] visible.
    ///
    /// If the target rect is larger than viewport size, we will prioritize
    /// the region of the target closest to its origin.
    pub fn pan_to_visible(&mut self, rect: Rect) -> bool {
        /// Given a position and the min and max edges of an axis,
        /// return a delta by which to adjust that axis such that the value
        /// falls between its edges.
        ///
        /// if the value already falls between the two edges, return 0.0.
        fn closest_on_axis(val: f64, min: f64, max: f64) -> f64 {
            assert!(min <= max);
            if val > min && val < max {
                0.0
            } else if val <= min {
                val - min
            } else {
                val - max
            }
        }

        // clamp the target region size to our own size.
        // this means we will show the portion of the target region that
        // includes the origin.
        let target_size = Size::new(
            rect.width().min(self.view_size.width),
            rect.height().min(self.view_size.height),
        );
        let rect = rect.with_size(target_size);

        let my_rect = self.view_rect();
        let x0 = closest_on_axis(rect.min_x(), my_rect.min_x(), my_rect.max_x());
        let x1 = closest_on_axis(rect.max_x(), my_rect.min_x(), my_rect.max_x());
        let y0 = closest_on_axis(rect.min_y(), my_rect.min_y(), my_rect.max_y());
        let y1 = closest_on_axis(rect.max_y(), my_rect.min_y(), my_rect.max_y());

        let delta_x = if x0.abs() > x1.abs() { x0 } else { x1 };
        let delta_y = if y0.abs() > y1.abs() { y0 } else { y1 };
        let new_origin = self.view_origin + Vec2::new(delta_x, delta_y);
        self.pan_to(new_origin)
    }
}

impl Viewport {
    /// Transform the event for the contents of a scrolling container.
    ///
    /// the `force` flag is used to ensure an event is delivered even
    /// if the cursor is out of the viewport, such as if the contents are active
    /// or hot.
    pub fn transform_event(&self, event: &Event, force: bool) -> Option<Event> {
        let offset = self.view_origin.to_vec2();
        let viewport_rect = self.content_size.to_rect();
        match event {
            Event::MouseDown(mouse_event) => {
                if force || viewport_rect.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseDown(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseUp(mouse_event) => {
                if force || viewport_rect.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseUp(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseMove(mouse_event) => {
                if force || viewport_rect.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseMove(mouse_event))
                } else {
                    None
                }
            }
            Event::Wheel(mouse_event) => {
                if force || viewport_rect.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::Wheel(mouse_event))
                } else {
                    None
                }
            }
            _ => Some(event.clone()),
        }
    }
}

/// A widget exposing a rectangular view into its child, which can be used as a building block for
/// widgets that scroll their child.
pub struct ClipBox<W> {
    pub child: WidgetPod<W>,
    port: Viewport,
    constrain_horizontal: bool,
    constrain_vertical: bool,
    must_fill: bool,
}

impl<W> ClipBox<W> {
    /// Builder-style method for deciding whether to constrain the child vertically.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a `ClipBox` lays out its child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its height: the idea is that the child can be as tall as it
    ///   wants, and the viewport will somehow get moved around to see all of it.
    /// - When it is `true`, the viewport's maximum height will be passed down
    ///   as an upper bound on the height of the child, and the viewport will set
    ///   its own height to be the same as its child's height.
    pub fn constrain_vertical(mut self, constrain: bool) -> Self {
        self.constrain_vertical = constrain;
        self
    }

    /// Builder-style method for deciding whether to constrain the child horizontally.
    ///
    /// The default is `false`. See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Builder-style method to set whether the child must fill the view.
    ///
    /// If `false` (the default) there is no minimum constraint on the child's
    /// size. If `true`, the child is passed the same minimum constraints as
    /// the `ClipBox`.
    pub fn content_must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
        self
    }

    /// Returns a the viewport describing this `ClipBox`'s position.
    pub fn viewport(&self) -> Viewport {
        self.port
    }

    /// Returns the origin of the viewport rectangle.
    pub fn viewport_origin(&self) -> Point {
        self.port.view_origin
    }

    /// Returns the size of the rectangular viewport into the child widget.
    /// To get the position of the viewport, see [`viewport_origin`].
    ///
    /// [`viewport_origin`]: struct.ClipBox.html#method.viewport_origin
    pub fn viewport_size(&self) -> Size {
        self.port.view_size
    }

    /// Returns the size of the child widget.
    pub fn content_size(&self) -> Size {
        self.port.content_size
    }

    /// Set whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.constrain_horizontal = constrain;
    }

    /// Set whether to constrain the child vertically.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.constrain_vertical = constrain;
    }

    /// Set whether the child's size must be greater than or equal the size of
    /// the `ClipBox`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: ClipBox::content_must_fill
    pub fn set_content_must_fill(&mut self, must_fill: bool) {
        self.must_fill = must_fill;
    }
}

impl<'a, 'b, W: Widget> WidgetView<'a, 'b, ClipBox<W>> {
    pub fn get_child_view(&mut self) -> WidgetView<'_, 'b, W> {
        WidgetView {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: &mut self.widget.child.state,
            widget: &mut self.widget.child.inner,
        }
    }
}

impl<W: Widget> ClipBox<W> {
    /// Creates a new `ClipBox` wrapping `child`.
    pub fn new(child: W) -> Self {
        ClipBox {
            child: WidgetPod::new(child),
            port: Default::default(),
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
        }
    }

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.child.widget()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.child.widget_mut()
    }

    /// Changes the viewport offset by `delta`.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn pan_by(&mut self, delta: Vec2) -> bool {
        self.pan_to(self.viewport_origin() + delta)
    }

    /// Changes the viewport offset on the specified axis to 'position'.
    ///
    /// The other axis will remain unchanged.
    pub fn pan_to_on_axis(&mut self, axis: Axis, position: f64) -> bool {
        self.pan_to(
            axis.pack(position, axis.minor_pos(self.viewport_origin()))
                .into(),
        )
    }

    /// Sets the viewport origin to `pos`.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, origin: Point) -> bool {
        if self.port.pan_to(origin) {
            self.child
                .set_viewport_offset(self.viewport_origin().to_vec2());
            true
        } else {
            false
        }
    }

    /// Adjust the viewport to display as much of the target region as is possible.
    ///
    /// Returns `true` if the viewport changes.
    ///
    /// This will move the viewport the smallest distance that fully shows
    /// the target region. If the target region is larger than the viewport,
    /// we will display the portion that fits, prioritizing the portion closest
    /// to the origin.
    pub fn pan_to_visible(&mut self, region: Rect) -> bool {
        if self.port.pan_to_visible(region) {
            self.child
                .set_viewport_offset(self.viewport_origin().to_vec2());
            true
        } else {
            false
        }
    }

    /// Modify the `ClipBox`'s viewport rectangle with a closure.
    ///
    /// The provided callback function can modify its argument, and when it is
    /// done then this `ClipBox` will be modified to have the new viewport rectangle.
    pub fn with_port<F: FnOnce(&mut Viewport)>(&mut self, f: F) {
        f(&mut self.port);
        self.child
            .set_viewport_offset(self.viewport_origin().to_vec2());
    }
}

impl<W: Widget> Widget for ClipBox<W> {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        ctx.init();
        let force_event = self.child.is_hot() || self.child.has_active();
        if let Some(child_event) = self.viewport().transform_event(&event, force_event) {
            self.child.on_event(ctx, &child_event, env);
        }
        ctx.skip_child(&mut self.child);
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        ctx.init();
        match event {
            LifeCycle::RequestPanToChild(target_rect) => {
                self.port.pan_to_visible(*target_rect);
                ctx.request_layout();
            }
            _ => {}
        }
        self.child.lifecycle(ctx, event, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();

        let max_child_width = if self.constrain_horizontal {
            bc.max().width
        } else {
            f64::INFINITY
        };
        let max_child_height = if self.constrain_vertical {
            bc.max().height
        } else {
            f64::INFINITY
        };
        let min_child_size = if self.must_fill { bc.min() } else { Size::ZERO };
        let child_bc =
            BoxConstraints::new(min_child_size, Size::new(max_child_width, max_child_height));

        let content_size = self.child.layout(ctx, &child_bc, env);
        self.port.content_size = content_size;
        self.child.set_origin(ctx, env, Point::ORIGIN);

        self.port.view_size = bc.constrain(content_size);
        let new_offset = self.port.clamp_view_origin(self.viewport_origin());
        self.pan_to(new_offset);
        trace!("Computed sized: {}", self.viewport_size());
        self.viewport_size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        ctx.init();
        let viewport = ctx.size().to_rect();
        let offset = self.viewport_origin().to_vec2();
        ctx.with_save(|ctx| {
            ctx.clip(viewport);
            ctx.transform(Affine::translate(-offset));

            let mut visible = ctx.region().clone();
            visible += offset;
            ctx.with_child_ctx(visible, |ctx| self.child.paint_raw(ctx, env));
        });
    }

    fn children2(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        smallvec![self.child.as_dyn()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("ClipBox")
    }
}
