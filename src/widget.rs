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

mod box_constraints;
pub mod button;
mod contexts;
mod core;
//pub mod layout_observer;
//pub mod list;
pub mod piet_scene_helpers;
mod raw_event;
//pub mod scroll_view;
//pub mod text;
//pub mod vstack;

use std::any::Any;
use std::ops::{Deref, DerefMut};

use glazier::kurbo::{Rect, Size};
use vello::SceneBuilder;

pub use self::box_constraints::BoxConstraints;
use self::contexts::LifeCycleCx;
pub use self::contexts::{AccessCx, CxState, EventCx, LayoutCx, PaintCx, UpdateCx};
pub use self::core::Pod;
pub(crate) use self::core::{ChangeFlags, PodFlags, WidgetState};
pub use self::raw_event::{Event, LifeCycle};

/// A basic widget trait.
pub trait Widget {
    /// Handle an event.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or submitting
    /// a [`Command`].
    ///
    /// [`Event`]: enum.Event.html
    /// [`EventCtx`]: struct.EventCtx.html
    /// [`Command`]: struct.Command.html
    fn event(&mut self, cx: &mut EventCx, event: &Event);

    /// Handle a life cycle notification.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`LifeCycle`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    /// [`LifeCycleCx`]: struct.LifeCycleCx.html
    /// [`Command`]: struct.Command.html
    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle);

    /// Update the widget's appearance in response to a change in the app's
    /// [`Data`] or [`Env`].
    ///
    /// This method is called when requested by the view layer.
    /// When the appearance of the widget needs to be updated in response to
    /// these changes, you can call [`request_paint`] or [`request_layout`] on
    /// the provided [`UpdateCtx`] to schedule calls to [`paint`] and [`layout`]
    /// as required.
    ///
    /// This method may go around.
    ///
    /// [`Data`]: trait.Data.html
    /// [`Env`]: struct.Env.html
    /// [`UpdateCtx`]: struct.UpdateCtx.html
    /// [`env_changed`]: struct.UpdateCtx.html#method.env_changed
    /// [`env_key_changed`]: struct.UpdateCtx.html#method.env_changed
    /// [`request_paint`]: struct.UpdateCtx.html#method.request_paint
    /// [`request_layout`]: struct.UpdateCtx.html#method.request_layout
    /// [`layout`]: #tymethod.layout
    /// [`paint`]: #tymethod.paint
    fn update(&mut self, cx: &mut UpdateCx);

    /// Compute layout.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`WidgetPod::layout`] on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// compute layout, then call [`set_origin`] on each of its children.
    /// Finally, it should return the size of the container. The container
    /// can recurse in any order, which can be helpful to, for example, compute
    /// the size of non-flex widgets first, to determine the amount of space
    /// available for the flex widgets.
    ///
    /// For efficiency, a container should only invoke layout of a child widget
    /// once, though there is nothing enforcing this.
    ///
    /// The layout strategy is strongly inspired by Flutter.
    ///
    /// [`WidgetPod::layout`]: struct.WidgetPod.html#method.layout
    /// [`set_origin`]: struct.WidgetPod.html#method.set_origin
    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size;

    /// Update the accessibility tree.
    fn accessibility(&mut self, cx: &mut AccessCx);

    /// Paint the widget appearance.
    ///
    /// The [`PaintCtx`] derefs to something that implements the [`RenderContext`]
    /// trait, which exposes various methods that the widget can use to paint
    /// its appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children, or annotations (for example, scrollbars) by painting
    /// afterwards. In addition, they can apply masks and transforms on
    /// the render context, which is especially useful for scrolling.
    ///
    /// [`PaintCtx`]: struct.PaintCtx.html
    /// [`RenderContext`]: trait.RenderContext.html
    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder);

    /*
    #[doc(hidden)]
    /// Get the identity of the widget; this is basically only implemented by
    /// `IdentityWrapper`. Widgets should not implement this on their own.
    fn id(&self) -> Option<WidgetId> {
        None
    }
    */

    #[doc(hidden)]
    /// Get the (verbose) type name of the widget for debugging purposes.
    /// You should not override this method.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    #[doc(hidden)]
    /// Get the (abridged) type name of the widget for debugging purposes.
    /// You should not override this method.
    fn short_type_name(&self) -> &'static str {
        let name = self.type_name();
        name.split('<')
            .next()
            .unwrap_or(name)
            .split("::")
            .last()
            .unwrap_or(name)
    }

    /*
    #[doc(hidden)]
    /// From the current data, get a best-effort description of the state of
    /// this widget and its children for debugging purposes.
    fn debug_state(&self) -> DebugState {
        #![allow(unused_variables)]
        DebugState {
            display_name: self.short_type_name().to_string(),
            ..Default::default()
        }
    }
    */

    /// Computes max intrinsic/preferred dimension of a widget on the provided axis.
    ///
    /// Max intrinsic/preferred dimension is the dimension the widget could take, provided infinite
    /// constraint on that axis.
    ///
    /// If axis == Axis::Horizontal, widget is being asked to calculate max intrinsic width.
    /// If axis == Axis::Vertical, widget is being asked to calculate max intrinsic height.
    ///
    /// Box constraints must be honored in intrinsics computation.
    ///
    /// AspectRatioBox is an example where constraints are honored. If height is finite, max intrinsic
    /// width is *height * ratio*.
    /// Only when height is infinite, child's max intrinsic width is calculated.
    ///
    /// Intrinsic is a *could-be* value. It's the value a widget *could* have given infinite constraints.
    /// This does not mean the value returned by layout() would be the same.
    ///
    /// This method **must** return a finite value.
    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCx,
        bc: &BoxConstraints,
    ) -> f64 {
        match axis {
            Axis::Horizontal => self.layout(ctx, bc).width,
            Axis::Vertical => self.layout(ctx, bc).height,
        }
    }
}

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// The x axis
    Horizontal,
    /// The y axis
    Vertical,
}

pub trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;
}

impl<W: Widget + 'static> AnyWidget for W {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl Widget for Box<dyn AnyWidget> {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.deref_mut().event(cx, event);
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.deref_mut().lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.deref_mut().update(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        self.deref_mut().layout(cx, bc)
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        self.deref_mut().accessibility(cx);
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        self.deref_mut().paint(cx, builder);
    }
}

pub trait WidgetTuple {
    fn length(&self) -> usize;

    // Follows Panoramix; rethink to reduce allocation
    // Maybe SmallVec?
    fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget>;
}

macro_rules! impl_widget_tuple {
    ( $n: tt; $( $WidgetType:ident),* ; $( $index:tt ),* ) => {
        impl< $( $WidgetType: AnyWidget ),* > WidgetTuple for ( $( $WidgetType, )* ) {
            fn length(&self) -> usize {
                $n
            }

            fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget> {
                let mut v: Vec<&mut dyn AnyWidget> = Vec::with_capacity(self.length());
                $(
                v.push(&mut self.$index);
                )*
                v
            }

        }
    }
}

impl_widget_tuple!(1; W0; 0);
impl_widget_tuple!(2; W0, W1; 0, 1);
impl_widget_tuple!(3; W0, W1, W2; 0, 1, 2);
impl_widget_tuple!(4; W0, W1, W2, W3; 0, 1, 2, 3);
impl_widget_tuple!(5; W0, W1, W2, W3, W4; 0, 1, 2, 3, 4);
impl_widget_tuple!(6; W0, W1, W2, W3, W4, W5; 0, 1, 2, 3, 4, 5);
impl_widget_tuple!(7; W0, W1, W2, W3, W4, W5, W6; 0, 1, 2, 3, 4, 5, 6);
impl_widget_tuple!(8;
    W0, W1, W2, W3, W4, W5, W6, W7;
    0, 1, 2, 3, 4, 5, 6, 7
);
