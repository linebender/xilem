// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! The context types that are passed into various widget methods.

use std::any::Any;
use std::time::Duration;

use accesskit::{NodeBuilder, TreeUpdate};
use parley::{FontContext, LayoutContext};
use tracing::{trace, warn};

use crate::action::Action;
use crate::dpi::LogicalPosition;
use crate::promise::PromiseToken;
use crate::render_root::{RenderRootSignal, RenderRootState};
use crate::text2::TextBrush;
use crate::text_helpers::{ImeChangeSignal, TextFieldRegistration};
use crate::tree_arena::TreeArenaTokenMut;
use crate::widget::{CursorChange, WidgetMut, WidgetState};
use crate::{AllowRawMut, CursorIcon, Insets, Point, Rect, Size, Widget, WidgetId, WidgetPod};

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

/// A context provided inside of [`WidgetMut`].
///
/// When you declare a mutable reference type for your widget, methods of this type
/// will have access to a `WidgetCtx`. If that method mutates the widget in a way that
/// requires a later pass (for instance, if your widget has a `set_color` method),
/// you will need to signal that change in the pass (eg `request_paint`).
///
// TODO add tutorial - See https://github.com/linebender/xilem/issues/376
pub struct WidgetCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) parent_widget_state: &'a mut WidgetState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
}

/// A context provided to event handling methods of widgets.
///
/// Widgets should call [`request_paint`](Self::request_paint) whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
pub struct EventCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
    pub(crate) is_handled: bool,
    pub(crate) request_pan_to_child: Option<Rect>,
}

/// A context provided to the [`lifecycle`] method on widgets.
///
/// [`lifecycle`]: Widget::lifecycle
pub struct LifeCycleCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
    pub(crate) mouse_pos: Option<Point>,
}

/// A context passed to paint methods of widgets.
pub struct PaintCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
    /// The approximate depth in the tree at the time of painting.
    pub(crate) depth: u32,
    pub(crate) debug_paint: bool,
    pub(crate) debug_widget: bool,
}

pub struct AccessCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) widget_state_children: TreeArenaTokenMut<'a, WidgetState>,
    pub(crate) widget_children: TreeArenaTokenMut<'a, Box<dyn Widget>>,
    pub(crate) tree_update: &'a mut TreeUpdate,
    pub(crate) current_node: NodeBuilder,
    pub(crate) rebuild_all: bool,
    pub(crate) scale_factor: f64,
}

pub struct WorkerCtx<'a> {
    // TODO
    #[allow(dead_code)]
    pub(crate) global_state: &'a mut RenderRootState,
}

pub struct WorkerFn(pub Box<dyn FnOnce(WorkerCtx) + Send + 'static>);

// --- MARK: GETTERS ---
// Methods for all context types
impl_context_method!(
    WidgetCtx<'_>,
    EventCtx<'_>,
    LifeCycleCtx<'_>,
    LayoutCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// get the `WidgetId` of the current widget.
        pub fn widget_id(&self) -> WidgetId {
            self.widget_state.id
        }

        /// Skip iterating over the given child.
        ///
        /// Normally, container widgets are supposed to iterate over each of their
        /// child widgets in their methods. By default, the framework treats not
        /// doing so as a mistake, and panics if debug assertions are on.
        ///
        /// This tells the framework that a child was deliberately skipped.
        // TODO - see event flow tutorial - See https://github.com/linebender/xilem/issues/376
        pub fn skip_child(&self, child: &mut WidgetPod<impl Widget>) {
            self.get_child_state(child).mark_as_visited(true);
        }

        #[allow(dead_code)]
        /// Helper method to get a direct reference to a child widget from its WidgetPod.
        fn get_child<Child: Widget>(&self, child: &'_ WidgetPod<Child>) -> &'_ Child {
            let (child, _child_token) = self
                .widget_children
                .get_child(child.id().to_raw())
                .expect("get_child: child not found");
            child.as_dyn_any().downcast_ref::<Child>().unwrap()
        }

        #[allow(dead_code)]
        /// Helper method to get a direct reference to a child widget's WidgetState from its WidgetPod.
        fn get_child_state<Child: Widget>(&self, child: &'_ WidgetPod<Child>) -> &'_ WidgetState {
            let (child_state, _child_state_token) = self
                .widget_state_children
                .get_child(child.id().to_raw())
                .expect("get_child_state: child not found");
            child_state
        }
    }
);

// Methods for all mutable context types
impl_context_method!(
    WidgetCtx<'_>,
    EventCtx<'_>,
    LifeCycleCtx<'_>,
    LayoutCtx<'_>,
    {
        /// Helper method to get a mutable reference to a child widget's WidgetState from its WidgetPod.
        ///
        /// This one isn't defined for PaintCtx and AccessCtx because those contexts
        /// can't mutate WidgetState.
        fn get_child_state_mut<Child: Widget>(
            &mut self,
            child: &'_ mut WidgetPod<Child>,
        ) -> &'_ mut WidgetState {
            let (child_state, _child_state_token) = self
                .widget_state_children
                .get_child_mut(child.id().to_raw())
                .expect("get_child_state_mut: child not found");
            child_state
        }
    }
);

// --- MARK: GET LAYOUT ---
// Methods on all context types except LayoutCtx
// These methods access layout info calculated during the layout pass.
impl_context_method!(
    WidgetCtx<'_>,
    EventCtx<'_>,
    LifeCycleCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The layout size.
        ///
        /// This is the layout size as ultimately determined by the parent
        /// container, on the previous layout pass.
        ///
        /// Generally it will be the same as the size returned by the child widget's
        /// [`layout`] method.
        ///
        /// [`layout`]: Widget::layout
        pub fn size(&self) -> Size {
            self.widget_state.size()
        }

        pub fn layout_rect(&self) -> Rect {
            self.widget_state.layout_rect()
        }

        /// The origin of the widget in window coordinates, relative to the top left corner of the
        /// content area.
        pub fn window_origin(&self) -> Point {
            self.widget_state.window_origin()
        }

        /// Convert a point from the widget's coordinate space to the window's.
        ///
        /// The returned point is relative to the content area; it excludes window chrome.
        pub fn to_window(&self, widget_point: Point) -> Point {
            self.window_origin() + widget_point.to_vec2()
        }
    }
);

// --- MARK: GET STATUS ---
// Methods on all context types except LayoutCtx
// Access status information (hot/active/disabled/etc).
impl_context_method!(
    WidgetCtx<'_>,
    EventCtx<'_>,
    LifeCycleCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The "hot" (aka hover) status of a widget.
        ///
        /// A widget is "hot" when the mouse is hovered over it. Widgets will
        /// often change their appearance as a visual indication that they
        /// will respond to mouse interaction.
        ///
        /// The hot status is computed from the widget's layout rect. In a
        /// container hierarchy, all widgets with layout rects containing the
        /// mouse position have hot status.
        ///
        /// Discussion: there is currently some confusion about whether a
        /// widget can be considered hot when some other widget is active (for
        /// example, when clicking to one widget and dragging to the next).
        /// The documentation should clearly state the resolution.
        pub fn is_hot(&self) -> bool {
            self.widget_state.is_hot
        }

        /// The active status of a widget.
        ///
        /// Active status generally corresponds to a mouse button down. Widgets
        /// with behavior similar to a button will call [`set_active`](EventCtx::set_active) on mouse
        /// down and then up.
        ///
        /// When a widget is active, it gets mouse events even when the mouse
        /// is dragged away.
        pub fn is_active(&self) -> bool {
            self.widget_state.is_active
        }

        /// The focus status of a widget.
        ///
        /// Returns `true` if this specific widget is focused.
        /// To check if any descendants are focused use [`has_focus`].
        ///
        /// Focus means that the widget receives keyboard events.
        ///
        /// A widget can request focus using the [`request_focus`] method.
        /// It's also possible to register for automatic focus via [`register_for_focus`].
        ///
        /// If a widget gains or loses focus it will get a [`StatusChange::FocusChanged`] event.
        ///
        /// Only one widget at a time is focused. However due to the way events are routed,
        /// all ancestors of that widget will also receive keyboard events.
        ///
        /// [`request_focus`]: EventCtx::request_focus
        /// [`register_for_focus`]: LifeCycleCtx::register_for_focus
        /// [`StatusChange::FocusChanged`]: crate::StatusChange::FocusChanged
        /// [`has_focus`]: Self::has_focus
        pub fn is_focused(&self) -> bool {
            self.global_state.focused_widget == Some(self.widget_id())
        }

        /// The (tree) focus status of a widget.
        ///
        /// Returns `true` if either this specific widget or any one of its descendants is focused.
        /// To check if only this specific widget is focused use [`is_focused`](Self::is_focused).
        pub fn has_focus(&self) -> bool {
            self.widget_state.has_focus
        }

        /// The disabled state of a widget.
        ///
        /// Returns `true` if this widget or any of its ancestors is explicitly disabled.
        /// To make this widget explicitly disabled use [`set_disabled`].
        ///
        /// Disabled means that this widget should not change the state of the application. What
        /// that means is not entirely clear but in any it should not change its data. Therefore
        /// others can use this as a safety mechanism to prevent the application from entering an
        /// illegal state.
        /// For an example the decrease button of a counter of type `usize` should be disabled if the
        /// value is `0`.
        ///
        /// [`set_disabled`]: EventCtx::set_disabled
        pub fn is_disabled(&self) -> bool {
            self.widget_state.is_disabled()
        }

        /// Check is widget is stashed.
        ///
        /// **Note:** Stashed widgets are a WIP feature
        // FIXME - take stashed parents into account
        pub fn is_stashed(&self) -> bool {
            self.widget_state.is_stashed
        }
    }
);

// --- MARK: CURSOR ---
// Cursor-related impls.
impl_context_method!(EventCtx<'_>, {
    /// Set the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. If a child widget also sets a
    /// cursor, the child widget's cursor will take precedence. (If that isn't what you want, use
    /// [`override_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn set_cursor(&mut self, cursor: &CursorIcon) {
        trace!("set_cursor {:?}", cursor);
        self.widget_state.cursor_change = CursorChange::Set(*cursor);
    }

    /// Override the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. This will override the cursor
    /// preferences of a child widget. (If that isn't what you want, use [`set_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`set_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn override_cursor(&mut self, cursor: &CursorIcon) {
        trace!("override_cursor {:?}", cursor);
        self.widget_state.cursor_change = CursorChange::Override(*cursor);
    }

    /// Clear the cursor icon.
    ///
    /// This undoes the effect of [`set_cursor`] and [`override_cursor`].
    ///
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`set_cursor`]: EventCtx::set_cursor
    pub fn clear_cursor(&mut self) {
        trace!("clear_cursor");
        self.widget_state.cursor_change = CursorChange::Default;
    }
});

// --- MARK: WIDGET_MUT ---
// Methods to get a child WidgetMut from a parent.
impl<'a> WidgetCtx<'a> {
    /// Return a [`WidgetMut`] to a child widget.
    pub fn get_mut<'c, Child: Widget>(
        &'c mut self,
        child: &'c mut WidgetPod<Child>,
    ) -> WidgetMut<'c, Child> {
        let (child_state, child_state_token) = self
            .widget_state_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let (child, child_token) = self
            .widget_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let child_ctx = WidgetCtx {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: child_state,
            widget_state_children: child_state_token,
            widget_children: child_token,
        };
        WidgetMut {
            ctx: child_ctx,
            widget: child.as_mut_dyn_any().downcast_mut().unwrap(),
        }
    }
}

// TODO - It's not clear whether EventCtx should be able to create a WidgetMut.
// One of the examples currently uses that feature to change a child widget's color
// in reaction to mouse events, but we might want to address that use-case differently.
impl<'a> EventCtx<'a> {
    /// Return a [`WidgetMut`] to a child widget.
    pub fn get_mut<'c, Child: Widget>(
        &'c mut self,
        child: &'c mut WidgetPod<Child>,
    ) -> WidgetMut<'c, Child> {
        let (child_state, child_state_token) = self
            .widget_state_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let (child, child_token) = self
            .widget_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let child_ctx = WidgetCtx {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: child_state,
            widget_state_children: child_state_token,
            widget_children: child_token,
        };
        WidgetMut {
            ctx: child_ctx,
            widget: child.as_mut_dyn_any().downcast_mut().unwrap(),
        }
    }
}

// TODO - It's not clear whether LifeCycleCtx should be able to create a WidgetMut.
impl<'a> LifeCycleCtx<'a> {
    /// Return a [`WidgetMut`] to a child widget.
    pub fn get_mut<'c, Child: Widget>(
        &'c mut self,
        child: &'c mut WidgetPod<Child>,
    ) -> WidgetMut<'c, Child> {
        let (child_state, child_state_token) = self
            .widget_state_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let (child, child_token) = self
            .widget_children
            .get_child_mut(child.id().to_raw())
            .expect("get_mut: child not found");
        let child_ctx = WidgetCtx {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: child_state,
            widget_state_children: child_state_token,
            widget_children: child_token,
        };
        WidgetMut {
            ctx: child_ctx,
            widget: child.as_mut_dyn_any().downcast_mut().unwrap(),
        }
    }
}

// --- MARK: UPDATE FLAGS ---
// Methods on WidgetCtx, EventCtx, and LifeCycleCtx
impl_context_method!(WidgetCtx<'_>, EventCtx<'_>, LifeCycleCtx<'_>, {
    /// Request a [`paint`](crate::Widget::paint) pass.
    pub fn request_paint(&mut self) {
        trace!("request_paint");
        self.widget_state.needs_paint = true;
    }

    /// Request a layout pass.
    ///
    /// A Widget's [`layout`] method is always called when the widget tree
    /// changes, or the window is resized.
    ///
    /// If your widget would like to have layout called at any other time,
    /// (such as if it would like to change the layout of children in
    /// response to some event) it must call this method.
    ///
    /// [`layout`]: crate::Widget::layout
    pub fn request_layout(&mut self) {
        trace!("request_layout");
        self.widget_state.needs_layout = true;
    }

    pub fn request_accessibility_update(&mut self) {
        trace!("request_accessibility_update");
        self.widget_state.needs_accessibility_update = true;
        self.widget_state.request_accessibility_update = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        trace!("request_anim_frame");
        self.widget_state.request_anim = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        trace!("children_changed");
        self.widget_state.children_changed = true;
        self.widget_state.update_focus_chain = true;
        self.request_layout();
    }

    /// Indicate that a child is about to be removed from the tree.
    ///
    /// Container widgets should avoid dropping WidgetPods. Instead, they should
    /// pass them to this method.
    pub fn remove_child(&mut self, child: WidgetPod<impl Widget>) {
        // TODO - Send recursive event to child
        let id = child.id().to_raw();
        let _ = self
            .widget_state_children
            .remove_child(id)
            .expect("remove_child: child not found");
        let _ = self
            .widget_children
            .remove_child(id)
            .expect("remove_child: child not found");

        self.children_changed();
    }

    /// Set the disabled state for this widget.
    ///
    /// Setting this to `false` does not mean a widget is not still disabled; for instance it may
    /// still be disabled by an ancestor. See [`is_disabled`] for more information.
    ///
    /// Calling this method during [`LifeCycle::DisabledChanged`] has no effect.
    ///
    /// [`LifeCycle::DisabledChanged`]: crate::LifeCycle::DisabledChanged
    /// [`is_disabled`]: EventCtx::is_disabled
    pub fn set_disabled(&mut self, disabled: bool) {
        // widget_state.children_disabled_changed is not set because we want to be able to delete
        // changes that happened during DisabledChanged.
        self.widget_state.is_explicitly_disabled_new = disabled;
    }

    /// Mark child widget as stashed.
    ///
    /// **Note:** Stashed widgets are a WIP feature
    pub fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget>, stashed: bool) {
        self.get_child_state_mut(child).is_stashed = stashed;
        self.children_changed();
    }

    #[allow(unused)]
    /// Indicate that text input state has changed.
    ///
    /// A widget that accepts text input should call this anytime input state
    /// (such as the text or the selection) changes as a result of a non text-input
    /// event.
    pub fn invalidate_text_input(&mut self, event: ImeChangeSignal) {
        todo!("invalidate_text_input");
    }
});

// --- MARK: OTHER METHODS ---
// Methods on all context types except PaintCtx and AccessCtx
impl_context_method!(
    WidgetCtx<'_>,
    EventCtx<'_>,
    LifeCycleCtx<'_>,
    LayoutCtx<'_>,
    {
        /// Submit an [`Action`].
        ///
        /// Note: Actions are still a WIP feature.
        pub fn submit_action(&mut self, action: Action) {
            trace!("submit_action");
            self.global_state
                .signal_queue
                .push_back(RenderRootSignal::Action(action, self.widget_state.id));
        }

        /// Run the provided function in the background.
        ///
        /// The function takes a [`WorkerCtx`] which it can use to
        /// communicate with the main thread.
        pub fn run_in_background(
            &mut self,
            _background_task: impl FnOnce(WorkerCtx) + Send + 'static,
        ) {
            // TODO - Use RenderRootSignal::SpawnWorker
            todo!("run_in_background");
        }

        /// Run the provided function in the background, and send its result once it's done.
        ///
        /// The function takes a [`WorkerCtx`] which it can use to
        /// communicate with the main thread.
        ///
        /// Once the function returns, an [`Event::PromiseResult`](crate::Event::PromiseResult)
        /// is emitted with the return value.
        pub fn compute_in_background<T: Any + Send>(
            &mut self,
            _background_task: impl FnOnce(WorkerCtx) -> T + Send + 'static,
        ) -> PromiseToken<T> {
            // TODO - Use RenderRootSignal::SpawnWorker
            todo!("compute_in_background");
        }

        /// Request a timer event.
        ///
        /// The return value is a token, which can be used to associate the
        /// request with the event.
        pub fn request_timer(&mut self, _deadline: Duration) -> TimerToken {
            todo!("request_timer");
        }
    }
);

// FIXME - Remove
pub struct TimerToken;

impl EventCtx<'_> {
    /// Send a signal to parent widgets to scroll this widget into view.
    pub fn request_pan_to_this(&mut self) {
        self.request_pan_to_child = Some(self.widget_state.layout_rect());
    }

    /// Set the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](Self::is_active).
    pub fn set_active(&mut self, active: bool) {
        trace!("set_active({})", active);
        self.widget_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Set the event as "handled", which stops its propagation to other
    /// widgets.
    pub fn set_handled(&mut self) {
        trace!("set_handled");
        self.is_handled = true;
    }

    /// Determine whether the event has been handled by some other widget.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// Request keyboard focus.
    ///
    /// Because only one widget can be focused at a time, multiple focus requests
    /// from different widgets during a single event cycle means that the last
    /// widget that requests focus will override the previous requests.
    ///
    /// See [`is_focused`](Self::is_focused) for more information about focus.
    pub fn request_focus(&mut self) {
        trace!("request_focus");
        // We need to send the request even if we're currently focused,
        // because we may have a sibling widget that already requested focus
        // and we have no way of knowing that yet. We need to override that
        // to deliver on the "last focus request wins" promise.
        let id = self.widget_id();
        self.global_state.next_focused_widget = Some(id);
    }

    /// Transfer focus to the widget with the given `WidgetId`.
    ///
    /// See [`is_focused`](Self::is_focused) for more information about focus.
    pub fn set_focus(&mut self, target: WidgetId) {
        trace!("set_focus target={:?}", target);
        self.global_state.next_focused_widget = Some(target);
    }

    /// Give up focus.
    ///
    /// This should only be called by a widget that currently has focus.
    ///
    /// See [`is_focused`](Self::is_focused) for more information about focus.
    pub fn resign_focus(&mut self) {
        trace!("resign_focus");
        if self.has_focus() {
            self.global_state.next_focused_widget = None;
        } else {
            warn!(
                "resign_focus can only be called by the currently focused widget \
                 or one of its ancestors. ({:?})",
                self.widget_id()
            );
        }
    }
}

impl LifeCycleCtx<'_> {
    /// Registers a child widget.
    ///
    /// This should only be called in response to a `LifeCycle::WidgetAdded` event.
    ///
    /// In general, you should not need to call this method; it is handled by
    /// the `WidgetPod`.
    // TODO - See https://github.com/linebender/xilem/issues/372
    pub(crate) fn register_child(&mut self, child_id: WidgetId) {
        trace!("register_child id={:?}", child_id);
        self.widget_state.children.add(&child_id);
    }

    /// Register this widget to be eligile to accept focus automatically.
    ///
    /// This should only be called in response to a [`LifeCycle::BuildFocusChain`] event.
    ///
    /// See [`EventCtx::is_focused`](Self::is_focused) for more information about focus.
    ///
    /// [`LifeCycle::BuildFocusChain`]: crate::LifeCycle::BuildFocusChain
    pub fn register_for_focus(&mut self) {
        trace!("register_for_focus");
        self.widget_state.focus_chain.push(self.widget_id());
    }

    /// Register this widget as accepting text input.
    pub fn register_as_text_input(&mut self) {
        let registration = TextFieldRegistration {
            widget_id: self.widget_id(),
        };
        self.widget_state.text_registrations.push(registration);
    }

    // TODO - remove - See issue https://github.com/linebender/xilem/issues/366
    /// Register this widget as a portal.
    ///
    /// This should only be used by scroll areas.
    pub fn register_as_portal(&mut self) {
        self.widget_state.is_portal = true;
    }
}

// --- MARK: UPDATE LAYOUT ---
impl LayoutCtx<'_> {
    fn assert_layout_done(&self, child: &WidgetPod<impl Widget>, method_name: &str) {
        if self.get_child_state(child).needs_layout {
            debug_panic!(
                "Error in #{}: trying to call '{}' with child '{}' #{} before computing its layout",
                self.widget_id().to_raw(),
                method_name,
                self.get_child(child).short_type_name(),
                child.id().to_raw(),
            );
        }
    }

    fn assert_placed(&self, child: &WidgetPod<impl Widget>, method_name: &str) {
        if self.get_child_state(child).is_expecting_place_child_call {
            debug_panic!(
                "Error in #{}: trying to call '{}' with child '{}' #{} before placing it",
                self.widget_id().to_raw(),
                method_name,
                self.get_child(child).short_type_name(),
                child.id().to_raw(),
            );
        }
    }

    /// Set explicit paint [`Insets`] for this widget.
    ///
    /// You are not required to set explicit paint bounds unless you need
    /// to paint outside of your layout bounds. In this case, the argument
    /// should be an [`Insets`] struct that indicates where your widget
    /// needs to overpaint, relative to its bounds.
    ///
    /// For more information, see [`WidgetPod::paint_insets`].
    ///
    /// [`WidgetPod::paint_insets`]: crate::widget::WidgetPod::paint_insets
    pub fn set_paint_insets(&mut self, insets: impl Into<Insets>) {
        let insets = insets.into();
        trace!("set_paint_insets {:?}", insets);
        self.widget_state.paint_insets = insets.nonnegative();
    }

    // TODO - This is currently redundant with the code in LayoutCtx::place_child
    /// Given a child and its parent's size, determine the
    /// appropriate paint `Insets` for the parent.
    ///
    /// This is a convenience method; it allows the parent to correctly
    /// propagate a child's desired paint rect, if it extends beyond the bounds
    /// of the parent's layout rect.
    ///
    /// ## Panics
    ///
    /// This method will panic if the child's [`layout()`](WidgetPod::layout) method has not been called yet
    /// and if [`LayoutCtx::place_child()`] has not been called for the child.
    pub fn compute_insets_from_child(
        &mut self,
        child: &WidgetPod<impl Widget>,
        my_size: Size,
    ) -> Insets {
        self.assert_layout_done(child, "compute_insets_from_child");
        self.assert_placed(child, "compute_insets_from_child");
        let parent_bounds = Rect::ZERO.with_size(my_size);
        let union_paint_rect = self
            .get_child_state(child)
            .paint_rect()
            .union(parent_bounds);
        union_paint_rect - parent_bounds
    }

    /// Set an explicit baseline position for this widget.
    ///
    /// The baseline position is used to align widgets that contain text,
    /// such as buttons, labels, and other controls. It may also be used
    /// by other widgets that are opinionated about how they are aligned
    /// relative to neighbouring text, such as switches or checkboxes.
    ///
    /// The provided value should be the distance from the *bottom* of the
    /// widget to the baseline.
    pub fn set_baseline_offset(&mut self, baseline: f64) {
        trace!("set_baseline_offset {}", baseline);
        self.widget_state.baseline_offset = baseline;
    }

    /// The distance from the bottom of the given widget to the baseline.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`WidgetPod::layout`] has not been called yet for
    /// the child.
    pub fn child_baseline_offset(&self, child: &WidgetPod<impl Widget>) -> f64 {
        self.assert_layout_done(child, "child_baseline_offset");
        self.get_child_state(child).baseline_offset
    }

    /// Get the given child's layout rect.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`WidgetPod::layout`] and [`LayoutCtx::place_child`]
    /// have not been called yet for the child.
    pub fn child_layout_rect(&self, child: &WidgetPod<impl Widget>) -> Rect {
        self.assert_layout_done(child, "child_layout_rect");
        self.assert_placed(child, "child_layout_rect");
        self.get_child_state(child).layout_rect()
    }

    /// Get the given child's paint rect.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`WidgetPod::layout`] and [`LayoutCtx::place_child`]
    /// have not been called yet for the child.
    pub fn child_paint_rect(&self, child: &WidgetPod<impl Widget>) -> Rect {
        self.assert_layout_done(child, "child_paint_rect");
        self.assert_placed(child, "child_paint_rect");
        self.get_child_state(child).paint_rect()
    }

    /// Get the given child's size.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`WidgetPod::layout`] has not been called yet for
    /// the child.
    pub fn child_size(&self, child: &WidgetPod<impl Widget>) -> Size {
        self.assert_layout_done(child, "child_size");
        self.get_child_state(child).layout_rect().size()
    }

    /// Set the position of a child widget, in the paren't coordinate space. This
    /// will also implicitly change "hot" status and affect the parent's display rect.
    ///
    /// Container widgets must call this method with each non-stashed child in their
    /// layout method, after calling `child.layout(...)`.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`WidgetPod::layout`] has not been called yet for
    /// the child.
    pub fn place_child<W: Widget>(&mut self, child: &mut WidgetPod<W>, origin: Point) {
        self.assert_layout_done(child, "place_child");
        if origin != self.get_child_state_mut(child).origin {
            self.get_child_state_mut(child).origin = origin;
            self.get_child_state_mut(child).needs_window_origin = true;
        }
        self.get_child_state_mut(child)
            .is_expecting_place_child_call = false;

        self.widget_state.local_paint_rect = self
            .widget_state
            .local_paint_rect
            .union(self.get_child_state(child).paint_rect());

        let child_id = child.id();
        let (child, child_token) = self
            .widget_children
            .get_child_mut(child_id.to_raw())
            .expect("place_child: child not found");
        let (child_state, child_state_token) = self
            .widget_state_children
            .get_child_mut(child_id.to_raw())
            .expect("place_child: child not found");
        let mouse_pos = self.mouse_pos.map(|pos| LogicalPosition::new(pos.x, pos.y));
        // if the widget has moved, it may have moved under the mouse, in which
        // case we need to handle that.
        if WidgetPod::update_hot_state(
            child_id,
            child.as_mut_dyn_any().downcast_mut::<W>().unwrap(),
            child_token,
            child_state,
            child_state_token,
            self.global_state,
            mouse_pos,
        ) {
            self.widget_state.merge_up(child_state);
        }
    }
}

// --- MARK: OTHER STUFF ---
impl_context_method!(LayoutCtx<'_>, PaintCtx<'_>, {
    /// Get the contexts needed to build and paint text sections.
    pub fn text_contexts(&mut self) -> (&mut FontContext, &mut LayoutContext<TextBrush>) {
        (
            &mut self.global_state.font_context,
            &mut self.global_state.text_layout_context,
        )
    }
});

impl PaintCtx<'_> {
    /// The depth in the tree of the currently painting widget.
    ///
    /// This may be used in combination with [`paint_with_z_index`](Self::paint_with_z_index) in order
    /// to correctly order painting operations.
    ///
    /// The `depth` here may not be exact; it is only guaranteed that a child will
    /// have a greater depth than its parent.
    #[inline]
    pub fn depth(&self) -> u32 {
        self.depth
    }

    // signal may be useful elsewhere, but is currently only used on PaintCtx
    /// Submit a [`RenderRootSignal`]
    ///
    /// Note: May be removed in future, and replaced with more specific methods.
    pub fn signal(&mut self, s: RenderRootSignal) {
        self.global_state.signal_queue.push_back(s);
    }
}

impl AccessCtx<'_> {
    pub fn current_node(&mut self) -> &mut NodeBuilder {
        &mut self.current_node
    }

    /// Report whether accessibility was requested on this widget.
    ///
    /// This method is primarily intended for containers. The `accessibility`
    /// method will be called on a widget when it or any of its descendants
    /// have seen a request. However, in many cases a container need not push
    /// a node for itself.
    pub fn is_requested(&self) -> bool {
        self.widget_state.needs_accessibility_update
    }
}

// --- MARK: RAW WRAPPERS ---
macro_rules! impl_get_raw {
    ($SomeCtx:tt) => {
        impl<'s> $SomeCtx<'s> {
            /// Get a child context and a raw shared reference to a child widget.
            ///
            /// The child context can be used to call context methods on behalf of the
            /// child widget.
            pub fn get_raw_ref<'a, 'r, Child: Widget>(
                &'a mut self,
                child: &'a mut WidgetPod<Child>,
            ) -> RawWrapper<'r, $SomeCtx<'r>, Child>
            where
                'a: 'r,
                's: 'r,
            {
                let (child_state, child_state_token) = self
                    .widget_state_children
                    .get_child_mut(child.id().to_raw())
                    .expect("get_raw_ref: child not found");
                let (child, child_token) = self
                    .widget_children
                    .get_child_mut(child.id().to_raw())
                    .expect("get_raw_ref: child not found");
                #[allow(clippy::needless_update)]
                let child_ctx = $SomeCtx {
                    widget_state: child_state,
                    widget_state_children: child_state_token,
                    widget_children: child_token,
                    global_state: self.global_state,
                    ..*self
                };
                RawWrapper {
                    ctx: child_ctx,
                    widget: child.as_dyn_any().downcast_ref().unwrap(),
                }
            }

            /// Get a raw mutable reference to a child widget.
            ///
            /// See documentation for [`AllowRawMut`] for more details.
            pub fn get_raw_mut<'a, 'r, Child: Widget + AllowRawMut>(
                &'a mut self,
                child: &'a mut WidgetPod<Child>,
            ) -> RawWrapperMut<'r, $SomeCtx<'r>, Child>
            where
                'a: 'r,
                's: 'r,
            {
                let (child_state, child_state_token) = self
                    .widget_state_children
                    .get_child_mut(child.id().to_raw())
                    .expect("get_raw_mut: child not found");
                let (child, child_token) = self
                    .widget_children
                    .get_child_mut(child.id().to_raw())
                    .expect("get_raw_mut: child not found");
                #[allow(clippy::needless_update)]
                let child_ctx = $SomeCtx {
                    widget_state: child_state,
                    widget_state_children: child_state_token,
                    widget_children: child_token,
                    global_state: self.global_state,
                    ..*self
                };
                RawWrapperMut {
                    parent_widget_state: &mut self.widget_state,
                    ctx: child_ctx,
                    widget: child.as_mut_dyn_any().downcast_mut().unwrap(),
                }
            }
        }
    };
}

impl_get_raw!(EventCtx);
impl_get_raw!(LifeCycleCtx);
impl_get_raw!(LayoutCtx);

impl<'s> AccessCtx<'s> {
    pub fn get_raw_ref<'a, 'r, Child: Widget>(
        &'a mut self,
        child: &'a WidgetPod<Child>,
    ) -> RawWrapper<'r, AccessCtx<'r>, Child>
    where
        'a: 'r,
        's: 'r,
    {
        let (child_state, child_state_token) = self
            .widget_state_children
            .get_child_mut(child.id().to_raw())
            .expect("get_raw_ref: child not found");
        let (child, child_token) = self
            .widget_children
            .get_child_mut(child.id().to_raw())
            .expect("get_raw_ref: child not found");
        let child_ctx = AccessCtx {
            widget_state: child_state,
            widget_state_children: child_state_token,
            widget_children: child_token,
            global_state: self.global_state,
            tree_update: self.tree_update,
            // TODO - This doesn't make sense. NodeBuilder should probably be split
            // out from AccessCtx.
            current_node: NodeBuilder::default(),
            rebuild_all: self.rebuild_all,
            scale_factor: self.scale_factor,
        };
        RawWrapper {
            ctx: child_ctx,
            widget: child.as_dyn_any().downcast_ref().unwrap(),
        }
    }
}

pub struct RawWrapper<'a, Ctx, W> {
    ctx: Ctx,
    widget: &'a W,
}

pub struct RawWrapperMut<'a, Ctx: IsContext, W> {
    parent_widget_state: &'a mut WidgetState,
    ctx: Ctx,
    widget: &'a mut W,
}

impl<Ctx, W> RawWrapper<'_, Ctx, W> {
    pub fn widget(&self) -> &W {
        self.widget
    }

    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }
}

impl<Ctx: IsContext, W> RawWrapperMut<'_, Ctx, W> {
    pub fn widget(&mut self) -> &mut W {
        self.widget
    }

    pub fn ctx(&mut self) -> &mut Ctx {
        &mut self.ctx
    }
}

impl<'a, Ctx: IsContext, W> Drop for RawWrapperMut<'a, Ctx, W> {
    fn drop(&mut self) {
        self.parent_widget_state
            .merge_up(self.ctx.get_widget_state());
    }
}

mod private {
    pub trait Sealed {}
}

pub trait IsContext: private::Sealed {
    fn get_widget_state(&mut self) -> &mut WidgetState;
}

macro_rules! impl_context_trait {
    ($SomeCtx:tt) => {
        impl private::Sealed for $SomeCtx<'_> {}

        impl IsContext for $SomeCtx<'_> {
            fn get_widget_state(&mut self) -> &mut WidgetState {
                self.widget_state
            }
        }
    };
}

impl_context_trait!(EventCtx);
impl_context_trait!(LifeCycleCtx);
impl_context_trait!(LayoutCtx);
