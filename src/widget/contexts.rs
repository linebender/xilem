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

//! Contexts for the widget system.
//!
//! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its contexts.rs.

use parley::FontContext;
use vello::kurbo::{Point, Size};

use super::{PodFlags, WidgetState};
use crate::Message;

// These contexts loosely follow Druid.

/// Static state that is shared between most contexts.
pub struct CxState<'a> {
    #[allow(unused)]
    font_cx: &'a mut FontContext,
    messages: &'a mut Vec<Message>,
}

/// A mutable context provided to [`event`] methods of widgets.
///
/// Widgets should call [`request_paint`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: EventCx::request_paint
/// [`event`]: crate::widget::Widget::event
pub struct EventCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) is_handled: bool,
}

/// A mutable context provided to the [`lifecycle`] method on widgets.
///
/// Certain methods on this context are only meaningful during the handling of
/// specific lifecycle events.
///
/// [`lifecycle`]: crate::widget::Widget::lifecycle
pub struct LifeCycleCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

/// A context passed to [`update`] methods of widgets.
///
/// Widgets should call [`request_paint`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: UpdateCx::request_paint
/// [`update`]: crate::widget::Widget::update
pub struct UpdateCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

/// A context passed to [`layout`] methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
///
/// [`layout`]: crate::widget::Widget::layout
pub struct LayoutCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

/// A context passed to [`paint`] methods of widgets.
///
/// [`paint`]: crate::widget::Widget::paint
pub struct PaintCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a WidgetState,
}

/// A macro for implementing methods on multiple contexts.
///
/// There are a lot of methods defined on multiple contexts; this lets us only
/// have to write them out once.
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        impl $ty { $($method)+ }
    };
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

impl<'a> CxState<'a> {
    pub fn new(font_cx: &'a mut FontContext, messages: &'a mut Vec<Message>) -> Self {
        CxState { font_cx, messages }
    }

    pub(crate) fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }
}

impl<'a, 'b> EventCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        EventCx {
            cx_state,
            widget_state: root_state,
            is_handled: false,
        }
    }

    /// Set the [`active`] state of the widget.
    ///
    /// [`active`]: Pod::is_active.
    pub fn set_active(&mut self, is_active: bool) {
        self.widget_state.flags.set(PodFlags::IS_ACTIVE, is_active);
    }

    /// Set the event as "handled", which stops its propagation to other
    /// widgets.
    pub fn set_handled(&mut self, is_handled: bool) {
        self.is_handled = is_handled;
    }

    /// Determine whether the event has been handled by some other widget.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }
}

impl<'a, 'b> LifeCycleCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LifeCycleCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b> UpdateCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        UpdateCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b> PaintCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, widget_state: &'a mut WidgetState) -> Self {
        PaintCx {
            cx_state,
            widget_state,
        }
    }
}

// Methods on all contexts.
//
// These Methods return information about the widget
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    PaintCx<'_, '_>,
    {
        /// Returns whether this widget is hot.
        ///
        /// See [`is_hot`] for more details.
        ///
        /// [`is_hot`]: super::Pod::is_hot
        pub fn is_hot(&self) -> bool {
            self.widget_state.flags.contains(PodFlags::IS_HOT)
        }

        /// Returns whether this widget is active.
        ///
        /// See [`is_active`] for more details.
        ///
        /// [`is_active`]: super::Pod::is_active
        pub fn is_active(&self) -> bool {
            self.widget_state.flags.contains(PodFlags::IS_ACTIVE)
        }
    }
);

// Methods on EventCx, UpdateCx, and LifeCycleCx
impl_context_method!(EventCx<'_, '_>, UpdateCx<'_, '_>, LifeCycleCx<'_, '_>, {
    /// Request layout for this widget.
    pub fn request_layout(&mut self) {
        self.widget_state.flags |= PodFlags::REQUEST_LAYOUT;
    }

    /// Sends a message to the view tree.
    ///
    /// Sending messages is the main way of interacting with views.
    /// Generally a Widget will send messages to its View after an interaction with the user. The
    /// view will schedule a rebuild if necessary and update the widget accordingly.
    /// Since widget can send messages to all views control widgets store the IdPath of their view
    /// to target them.
    //TODO: Decide whether it should be possible to send messages from Layout?
    pub fn add_message(&mut self, message: Message) {
        self.cx_state.messages.push(message);
    }
});

// Methods on EventCx, UpdateCx, LifeCycleCx and LayoutCx
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    {
        /// Requests a call to [`paint`] for this widget.
        ///
        /// [`paint`]: super::Widget::paint
        pub fn request_paint(&mut self) {
            self.widget_state.flags |= PodFlags::REQUEST_PAINT;
        }

        /// Notify Xilem that this widgets view context changed.
        ///
        /// A [`LifeCycle::ViewContextChanged`] event will be scheduled.
        /// Widgets only have to call this method in case they are changing the z-order of
        /// overlapping children or change the clip region all other changes are tracked internally.
        ///
        /// [`LifeCycle::ViewContextChanged`]: super::LifeCycle::ViewContextChanged
        pub fn view_context_changed(&mut self) {
            self.widget_state.flags |= PodFlags::VIEW_CONTEXT_CHANGED;
        }
    }
);
// Methods on LayoutCx and PaintCx
//
// These Methods return information about the widget
impl_context_method!(LayoutCx<'_, '_>, PaintCx<'_, '_>, {
    /// Returns a FontContext for creating TextLayouts.
    pub fn font_cx(&mut self) -> &mut FontContext {
        self.cx_state.font_cx
    }
});

// Methods on all contexts besides LayoutCx.
//
// These Methods return information about the widget
impl_context_method!(
    EventCx<'_, '_>,
    UpdateCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    LayoutCx<'_, '_>,
    PaintCx<'_, '_>,
    {
        /// The layout size.
        ///
        /// This is the layout size as ultimately determined by the parent
        /// container, on the previous layout pass.
        ///
        /// Generally it will be the same as the size returned by the child widget's
        /// [`layout`] method.
        ///
        /// [`layout`]: super::Widget::layout
        pub fn size(&self) -> Size {
            self.widget_state.size
        }

        /// The origin of the widget in window coordinates, relative to the top left corner of the
        /// content area.
        ///
        /// This value changes after calling [`set_origin`] on the [`Pod`] of this Widget or any of
        /// its ancestors.
        ///
        /// [`Pod`]: super::Pod
        /// [`set_origin`]: super::Pod::set_origin
        pub fn window_origin(&self) -> Point {
            self.widget_state.parent_window_origin + self.widget_state.origin.to_vec2()
        }
    }
);
