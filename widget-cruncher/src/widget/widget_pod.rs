use smallvec::SmallVec;
use std::collections::HashMap;
use tracing::{info_span, trace, warn};

use crate::bloom::Bloom;
use crate::contexts::ContextState;
use crate::kurbo::{Affine, Insets, Point, Rect, Shape, Size, Vec2};
use crate::text::{TextFieldRegistration, TextLayout};
use crate::util::ExtendDrain;
use crate::widget::{CursorChange, FocusChange, WidgetState};
use crate::{
    ArcStr, BoxConstraints, Color, Cursor, Env, Event, EventCtx, InternalEvent, InternalLifeCycle,
    LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Region, RenderContext, TimerToken, Widget,
    WidgetId,
};

// TODO
use crate::event::StatusChange;

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// `WidgetPod` will translate internal druid events to regular events,
/// synthesize additional events of interest, and stop propagation when it makes sense.
pub struct WidgetPod<W> {
    state: WidgetState,
    inner: W,
    env: Option<Env>,
    // stashed layout so we don't recompute this when debugging
    debug_widget_text: TextLayout<ArcStr>,
}

// Trait used to abstract over WidgetPods of any widget type.
pub trait AsWidgetPod {
    fn state(&self) -> &WidgetState;

    // FIXME - remove
    fn state_mut(&mut self) -> &mut WidgetState;

    // Return a reference to the inner widget.
    fn widget(&self) -> &dyn Widget;

    // FIXME - remove
    fn widget_mut(&mut self) -> &mut dyn Widget;

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]>;
    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]>;
    fn find_widget_by_id(&self, id: WidgetId) -> Option<&dyn AsWidgetPod>;
    fn find_widget_at_pos(&self, pos: Point) -> Option<&dyn AsWidgetPod>;
}

// ---

impl<W: Widget> WidgetPod<W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<W> {
        let mut state = WidgetState::new(WidgetId::next(), None);
        state.children_changed = true;
        state.needs_layout = true;
        WidgetPod {
            state,
            inner,
            env: None,
            debug_widget_text: TextLayout::new(),
        }
    }

    /// Create a new widget pod with fixed id.
    pub fn new_with_id(inner: W, id: WidgetId) -> WidgetPod<W> {
        let mut state = WidgetState::new(id, None);
        state.children_changed = true;
        state.needs_layout = true;
        WidgetPod {
            state,
            inner,
            env: None,
            debug_widget_text: TextLayout::new(),
        }
    }

    /// Read-only access to state. We don't mark the field as `pub` because
    /// we want to control mutation.
    pub(crate) fn state(&self) -> &WidgetState {
        &self.state
    }

    /// Return a reference to the inner widget.
    pub fn widget(&self) -> &W {
        &self.inner
    }

    /// Return a mutable reference to the inner widget.
    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns `true` if the widget has received [`LifeCycle::WidgetAdded`].
    ///
    /// [`LifeCycle::WidgetAdded`]: ./enum.LifeCycle.html#variant.WidgetAdded
    pub fn is_initialized(&self) -> bool {
        !self.state.is_new
    }

    /// Returns `true` if widget or any descendent is focused
    pub fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    /// Query the "active" state of the widget.
    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    /// Returns `true` if any descendant is active.
    pub fn has_active(&self) -> bool {
        self.state.has_active
    }

    /// Query the "hot" state of the widget.
    ///
    /// See [`EventCtx::is_hot`](struct.EventCtx.html#method.is_hot) for
    /// additional information.
    pub fn is_hot(&self) -> bool {
        self.state.is_hot
    }

    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.state.id
    }

    /// Set the origin of this widget, in the parent's coordinate space.
    ///
    /// A container widget should call the [`Widget::layout`] method on its children in
    /// its own [`Widget::layout`] implementation, and then call `set_origin` to
    /// position those children.
    ///
    /// The child will receive the [`LifeCycle::Size`] event informing them of the final [`Size`].
    ///
    /// [`Widget::layout`]: trait.Widget.html#tymethod.layout
    /// [`Rect`]: struct.Rect.html
    /// [`Size`]: struct.Size.html
    /// [`LifeCycle::Size`]: enum.LifeCycle.html#variant.Size
    pub fn set_origin(&mut self, ctx: &mut LayoutCtx, env: &Env, origin: Point) {
        self.state.origin = origin;
        self.state.is_expecting_set_origin_call = false;
        let layout_rect = self.layout_rect();

        // if the widget has moved, it may have moved under the mouse, in which
        // case we need to handle that.
        if WidgetPod::update_hot_state(
            &mut self.inner,
            &mut self.state,
            ctx.global_state,
            layout_rect,
            ctx.mouse_pos,
            env,
        ) {
            ctx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Returns the layout [`Rect`].
    ///
    /// This will be a [`Rect`] with a [`Size`] determined by the child's [`layout`]
    /// method, and the origin that was set by [`set_origin`].
    ///
    /// [`Rect`]: struct.Rect.html
    /// [`Size`]: struct.Size.html
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`set_origin`]: WidgetPod::set_origin
    pub fn layout_rect(&self) -> Rect {
        self.state.layout_rect()
    }

    /// Set the viewport offset.
    ///
    /// This is relevant only for children of a scroll view (or similar). It must
    /// be set by the parent widget whenever it modifies the position of its child
    /// while painting it and propagating events. As a rule of thumb, you need this
    /// if and only if you `Affine::translate` the paint context before painting
    /// your child. For an example, see the implementation of [`Scroll`].
    ///
    /// [`Scroll`]: widget/struct.Scroll.html
    pub fn set_viewport_offset(&mut self, offset: Vec2) {
        if offset != self.state.viewport_offset {
            self.state.needs_window_origin = true;
        }
        self.state.viewport_offset = offset;
    }

    /// The viewport offset.
    ///
    /// This will be the same value as set by [`set_viewport_offset`].
    ///
    /// [`set_viewport_offset`]: #method.viewport_offset
    pub fn viewport_offset(&self) -> Vec2 {
        self.state.viewport_offset
    }

    /// Get the widget's paint [`Rect`].
    ///
    /// This is the [`Rect`] that widget has indicated it needs to paint in.
    /// This is the same as the [`layout_rect`] with the [`paint_insets`] applied;
    /// in the general case it is the same as the [`layout_rect`].
    ///
    /// [`layout_rect`]: #method.layout_rect
    /// [`Rect`]: struct.Rect.html
    /// [`paint_insets`]: #method.paint_insets
    pub fn paint_rect(&self) -> Rect {
        self.state.paint_rect()
    }

    /// Return the paint [`Insets`] for this widget.
    ///
    /// If these [`Insets`] are nonzero, they describe the area beyond a widget's
    /// layout rect where it needs to paint.
    ///
    /// These are generally zero; exceptions are widgets that do things like
    /// paint a drop shadow.
    ///
    /// A widget can set its insets by calling [`set_paint_insets`] during its
    /// [`layout`] method.
    ///
    /// [`Insets`]: struct.Insets.html
    /// [`set_paint_insets`]: struct.LayoutCtx.html#method.set_paint_insets
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn paint_insets(&self) -> Insets {
        self.state.paint_insets
    }

    /// Given a parents layout size, determine the appropriate paint `Insets`
    /// for the parent.
    ///
    /// This is a convenience method to be used from the [`layout`] method
    /// of a `Widget` that manages a child; it allows the parent to correctly
    /// propogate a child's desired paint rect, if it extends beyond the bounds
    /// of the parent's layout rect.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`Insets`]: struct.Insets.html
    pub fn compute_parent_paint_insets(&self, parent_size: Size) -> Insets {
        let parent_bounds = Rect::ZERO.with_size(parent_size);
        let union_pant_rect = self.paint_rect().union(parent_bounds);
        union_pant_rect - parent_bounds
    }

    /// The distance from the bottom of this widget to the baseline.
    pub fn baseline_offset(&self) -> f64 {
        self.state.baseline_offset
    }
}

impl<W: Widget> WidgetPod<W> {
    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<&dyn AsWidgetPod> {
        if self.id() == id {
            Some(self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    pub fn find_widget_at_pos(&self, pos: Point) -> Option<&dyn AsWidgetPod> {
        let mut pos = pos;
        let mut innermost_widget: &dyn AsWidgetPod = self;

        if !self.state.layout_rect().contains(pos) {
            return None;
        }

        // FIXME - Handle hidden widgets (eg in scroll areas).
        loop {
            if let Some(child) = innermost_widget.widget().get_child_at_pos(pos) {
                pos -= innermost_widget.state().layout_rect().origin().to_vec2();
                innermost_widget = child;
            } else {
                return Some(innermost_widget);
            }
        }
    }
}

impl<W: Widget> WidgetPod<W> {
    #[inline(always)]
    pub(crate) fn mark_as_visited(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.state.was_visited = true;
        }
    }

    /// Determines if the provided `mouse_pos` is inside `rect`
    /// and if so updates the hot state and sends `LifeCycle::HotChanged`.
    ///
    /// Returns `true` if the hot state changed.
    ///
    /// The provided `child_state` should be merged up if this returns `true`.
    fn update_hot_state(
        inner: &mut W,
        inner_state: &mut WidgetState,
        global_state: &mut ContextState,
        rect: Rect,
        mouse_pos: Option<Point>,
        env: &Env,
    ) -> bool {
        let had_hot = inner_state.is_hot;
        inner_state.is_hot = match mouse_pos {
            Some(pos) => rect.winding(pos) != 0,
            None => false,
        };
        trace!(
            "Widget {:?}: set hot state to {}",
            inner_state.id,
            inner_state.is_hot
        );
        // FIXME - don't send event, update flags instead
        if had_hot != inner_state.is_hot {
            let hot_changed_event = StatusChange::HotChanged(inner_state.is_hot);
            let mut inner_ctx = LifeCycleCtx {
                global_state,
                widget_state: inner_state,
            };
            // We add a span so that inner logs are marked as being in a lifecycle pass
            info_span!("lifecycle")
                .in_scope(|| inner.on_status_change(&mut inner_ctx, &hot_changed_event, env));
            // if hot changes and we're showing widget ids, always repaint
            if env.get(Env::DEBUG_WIDGET_ID) {
                inner_ctx.request_paint();
            }
            return true;
        }
        false
    }

    fn recurse_pass<Ret>(
        &mut self,
        pass_name: &str,
        parent_state: &mut WidgetState,
        visit: impl FnOnce(&mut W, &mut WidgetState) -> Ret,
    ) -> Ret {
        let res = visit(&mut self.inner, &mut self.state);
        parent_state.merge_up(&mut self.state);
        res
    }

    #[inline(always)]
    fn call_widget_method_with_checks<Ret>(
        &mut self,
        method_name: &str,
        visit: impl FnOnce(&mut Self) -> Ret,
    ) -> Ret {
        #[cfg(debug_assertions)]
        for child in self.inner.children_mut() {
            child.state_mut().was_visited = false;
        }

        let return_value = visit(self);

        #[cfg(debug_assertions)]
        for child in self.inner.children() {
            if !child.state().was_visited {
                debug_panic!(
                    "Error in '{}' #{}: child widget '{}' #{} not visited in method {}",
                    self.widget().short_type_name(),
                    self.state().id.to_raw(),
                    child.widget().short_type_name(),
                    child.state().id.to_raw(),
                    method_name,
                )
            }
        }

        return_value
    }

    fn check_initialized(&self, method_name: &str) {
        if !self.is_initialized() {
            debug_panic!(
                "Error in '{}' #{}: method '{}' called before receiving WidgetAdded.",
                self.inner.short_type_name(),
                self.state.id.to_raw(),
                method_name,
            );
        }
    }
}

impl<W: Widget + 'static> WidgetPod<W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> WidgetPod<Box<dyn Widget>> {
        WidgetPod::new(Box::new(self.inner))
    }
}

// --- TRAIT IMPLS ---

impl<W: Widget> WidgetPod<W> {
    /// --- ON_EVENT ---

    /// Propagate an event.
    ///
    /// Generally the [`event`] method of a container widget will call this
    /// method on all its children. Here is where a great deal of the event
    /// flow logic resides, particularly whether to continue propagating
    /// the event.
    ///
    /// [`event`]: trait.Widget.html#tymethod.event
    pub fn on_event(&mut self, parent_ctx: &mut EventCtx, event: &Event, env: &Env) {
        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("on_event");

        // log if we seem not to be laid out when we should be
        if self.state.is_expecting_set_origin_call && !event.should_propagate_to_hidden() {
            warn!(
                "{:?} received an event ({:?}) without having been laid out. \
                This likely indicates a missed call to set_origin.",
                parent_ctx.widget_id(),
                event,
            );
        }

        if parent_ctx.is_handled {
            // If the event was already handled, we quit early.
            return;
        }

        let had_active = self.state.has_active;
        let rect = self.layout_rect();

        // If we need to replace either the event or its data.
        let mut modified_event = None;

        // TODO: factor as much logic as possible into monomorphic functions.
        let call_inner = match event {
            Event::Internal(internal) => match internal {
                InternalEvent::MouseLeave => {
                    let hot_changed = WidgetPod::update_hot_state(
                        &mut self.inner,
                        &mut self.state,
                        parent_ctx.global_state,
                        rect,
                        None,
                        env,
                    );
                    had_active || hot_changed
                }
                InternalEvent::RouteTimer(token, widget_id) => {
                    if *widget_id == self.id() {
                        modified_event = Some(Event::Timer(*token));
                        true
                    } else {
                        self.state.children.may_contain(widget_id)
                    }
                }
                InternalEvent::RouteImeStateChange(widget_id) => {
                    if *widget_id == self.id() {
                        modified_event = Some(Event::ImeStateChange);
                        true
                    } else {
                        self.state.children.may_contain(widget_id)
                    }
                }
            },
            Event::WindowConnected | Event::WindowCloseRequested => true,
            Event::WindowDisconnected => true,
            Event::WindowSize(_) => {
                self.state.needs_layout = true;
                parent_ctx.is_root
            }
            Event::MouseDown(mouse_event) => {
                WidgetPod::update_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    parent_ctx.global_state,
                    rect,
                    Some(mouse_event.pos),
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseDown(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseUp(mouse_event) => {
                WidgetPod::update_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    parent_ctx.global_state,
                    rect,
                    Some(mouse_event.pos),
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseUp(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseMove(mouse_event) => {
                let hot_changed = WidgetPod::update_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    parent_ctx.global_state,
                    rect,
                    Some(mouse_event.pos),
                    env,
                );
                // MouseMove is recursed even if the widget is not active and not hot,
                // but was hot previously. This is to allow the widget to respond to the movement,
                // e.g. drag functionality where the widget wants to follow the mouse.
                if had_active || self.state.is_hot || hot_changed {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseMove(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::Wheel(mouse_event) => {
                WidgetPod::update_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    parent_ctx.global_state,
                    rect,
                    Some(mouse_event.pos),
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::Wheel(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::AnimFrame(_) => {
                let r = self.state.request_anim;
                self.state.request_anim = false;
                r
            }
            Event::KeyDown(_) => self.state.has_focus,
            Event::KeyUp(_) => self.state.has_focus,
            Event::Paste(_) => self.state.has_focus,
            Event::Zoom(_) => had_active || self.state.is_hot,
            Event::Timer(_) => false, // This event was targeted only to our parent
            Event::ImeStateChange => true, // once delivered to the focus widget, recurse to the component?
            //Event::Command(_) => true,
            //Event::Notification(_) => false,
        };

        if call_inner {
            self.call_widget_method_with_checks("event", |widget_pod| {
                let mut inner_ctx = EventCtx {
                    global_state: parent_ctx.global_state,
                    widget_state: &mut widget_pod.state,
                    is_handled: false,
                    is_root: false,
                };
                let inner_event = modified_event.as_ref().unwrap_or(event);
                inner_ctx.widget_state.has_active = false;

                widget_pod.inner.on_event(&mut inner_ctx, inner_event, env);

                inner_ctx.widget_state.has_active |= inner_ctx.widget_state.is_active;
                parent_ctx.is_handled |= inner_ctx.is_handled;
            });
        } else {
            trace!("event wasn't propagated to {:?}", self.state.id);
        }

        // Always merge even if not needed, because merging is idempotent and gives us simpler code.
        // Doing this conditionally only makes sense when there's a measurable performance boost.
        parent_ctx.widget_state.merge_up(&mut self.state);
    }

    // --- LIFECYCLE ---

    /// Propagate a [`LifeCycle`] event.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    pub fn lifecycle(&mut self, parent_ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        // TODO - explain this
        self.mark_as_visited();

        // when routing a status change event, if we are at our target
        // we may send an extra event after the actual event
        let mut extra_event = None;

        let had_focus = self.state.has_focus;

        let call_inner = match event {
            LifeCycle::Internal(internal) => match internal {
                InternalLifeCycle::RouteWidgetAdded => {
                    // if this is called either we were just created, in
                    // which case we need to change lifecycle event to
                    // WidgetAdded or in case we were already created
                    // we just pass this event down
                    if self.state.is_new {
                        self.lifecycle(parent_ctx, &LifeCycle::WidgetAdded, env);
                        return;
                    } else {
                        if self.state.children_changed {
                            // Separate "widget removed" case.
                            self.state.children.clear();
                        }
                        self.state.children_changed
                    }
                }
                InternalLifeCycle::RouteDisabledChanged => {
                    self.state.update_focus_chain = true;

                    let was_disabled = self.state.is_disabled();

                    self.state.is_explicitly_disabled = self.state.is_explicitly_disabled_new;

                    if was_disabled != self.state.is_disabled() {
                        extra_event = Some(StatusChange::DisabledChanged(self.state.is_disabled()));
                        //Each widget needs only one of DisabledChanged and RouteDisabledChanged
                        false
                    } else {
                        self.state.children_disabled_changed
                    }
                }
                InternalLifeCycle::RouteFocusChanged { old, new } => {
                    let this_changed = if *old == Some(self.state.id) {
                        Some(false)
                    } else if *new == Some(self.state.id) {
                        Some(true)
                    } else {
                        None
                    };

                    if let Some(change) = this_changed {
                        self.state.has_focus = change;
                        extra_event = Some(StatusChange::FocusChanged(change));
                    } else {
                        self.state.has_focus = false;
                    }

                    // Recurse when the target widgets could be our descendants.
                    // The bloom filter we're checking can return false positives.
                    match (old, new) {
                        (Some(old), _) if self.state.children.may_contain(old) => true,
                        (_, Some(new)) if self.state.children.may_contain(new) => true,
                        _ => false,
                    }
                }
                InternalLifeCycle::ParentWindowOrigin => {
                    self.state.parent_window_origin = parent_ctx.widget_state.window_origin();
                    self.state.needs_window_origin = false;
                    true
                }
            },
            LifeCycle::WidgetAdded => {
                if self.state.is_new {
                    // TODO - better warning.
                    warn!("Already initialized.");
                }
                trace!(
                    "{} Received LifeCycle::WidgetAdded",
                    self.inner.short_type_name()
                );

                self.state.update_focus_chain = true;
                self.env = Some(env.clone());
                self.state.is_new = false;

                true
            }
            _ if !self.is_initialized() => {
                debug_panic!(
                    "Error in '{}' #{}: received LifeCycle::{:?} before receiving WidgetAdded.",
                    self.inner.short_type_name(),
                    self.state.id.to_raw(),
                    event
                );
                return;
            }
            /*
            LifeCycle::DisabledChanged(ancestors_disabled) => {
                self.state.update_focus_chain = true;

                let was_disabled = self.state.is_disabled();

                self.state.is_explicitly_disabled = self.state.is_explicitly_disabled_new;
                self.state.ancestor_disabled = *ancestors_disabled;

                // the change direction (true -> false or false -> true) of our parent and ourself
                // is always the same, or we dont change at all, because we stay disabled if either
                // we or our parent are disabled.
                was_disabled != self.state.is_disabled()
            }
            //NOTE: this is not sent here, but from the special update_hot_state method
            LifeCycle::HotChanged(_) => false,
            LifeCycle::FocusChanged(_) => {
                // We are a descendant of a widget that has/had focus.
                // Descendants don't inherit focus, so don't recurse.
                false
            }
            */
            LifeCycle::BuildFocusChain => {
                if self.state.update_focus_chain {
                    // Replace has_focus to check if the value changed in the meantime
                    let is_focused = parent_ctx.global_state.focus_widget == Some(self.state.id);
                    self.state.has_focus = is_focused;

                    self.state.focus_chain.clear();
                    true
                } else {
                    false
                }
            }
        };

        self.call_widget_method_with_checks("lifecycle", |widget_pod| {
            let mut inner_ctx = LifeCycleCtx {
                global_state: parent_ctx.global_state,
                widget_state: &mut widget_pod.state,
            };

            if call_inner {
                widget_pod.inner.lifecycle(&mut inner_ctx, event, env);
            }
        });

        let mut inner_ctx = LifeCycleCtx {
            global_state: parent_ctx.global_state,
            widget_state: &mut self.state,
        };

        if let Some(event) = extra_event.as_ref() {
            self.inner.on_status_change(&mut inner_ctx, event, env);
        }

        // Sync our state with our parent's state after the event!

        match event {
            // we need to (re)register children in case of one of the following events
            LifeCycle::WidgetAdded | LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded) => {
                self.state.children_changed = false;
                parent_ctx.widget_state.children =
                    parent_ctx.widget_state.children.union(self.state.children);
                parent_ctx.register_child(self.id());
            }
            //LifeCycle::DisabledChanged(_)
            LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged) => {
                self.state.children_disabled_changed = false;

                if self.state.is_disabled() && self.state.has_focus {
                    // This may gets overwritten. This is ok because it still ensures that a
                    // FocusChange is routed after we updated the focus-chain.
                    self.state.request_focus = Some(FocusChange::Resign);
                }

                // Delete changes of disabled state that happened during DisabledChanged to avoid
                // recursions.
                self.state.is_explicitly_disabled_new = self.state.is_explicitly_disabled;
            }
            // Update focus-chain of our parent
            LifeCycle::BuildFocusChain => {
                self.state.update_focus_chain = false;

                // had_focus is the old focus value. state.has_focus was repaced with parent_ctx.is_focused().
                // Therefore if had_focus is true but state.has_focus is false then the widget which is
                // currently focused is not part of the functional tree anymore
                // (Lifecycle::BuildFocusChain.should_propagate_to_hidden() is false!) and should
                // resign the focus.
                if had_focus && !self.state.has_focus {
                    self.state.request_focus = Some(FocusChange::Resign);
                }
                self.state.has_focus = had_focus;

                if !self.state.is_disabled() {
                    parent_ctx
                        .widget_state
                        .focus_chain
                        .extend(&self.state.focus_chain);
                }
            }
            _ => (),
        }

        parent_ctx.widget_state.merge_up(&mut self.state);
    }

    // --- LAYOUT ---

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn layout(&mut self, parent_ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("layout");

        self.state.needs_layout = false;
        self.state.needs_window_origin = false;
        self.state.is_expecting_set_origin_call = true;

        let inner_mouse_pos = parent_ctx
            .mouse_pos
            .map(|pos| pos - self.layout_rect().origin().to_vec2() + self.viewport_offset());
        let prev_size = self.state.size;

        let new_size = self.call_widget_method_with_checks("layout", |widget_pod| {
            // widget_pod is a reborrow of `self`

            let mut inner_ctx = LayoutCtx {
                widget_state: &mut widget_pod.state,
                global_state: parent_ctx.global_state,
                mouse_pos: inner_mouse_pos,
            };

            widget_pod.inner.layout(&mut inner_ctx, bc, env)
        });

        parent_ctx.widget_state.merge_up(&mut self.state);
        self.state.size = new_size;
        self.log_layout_issues(new_size);

        new_size
    }

    fn log_layout_issues(&self, size: Size) {
        if size.width.is_infinite() {
            let name = self.widget().type_name();
            warn!("Widget `{}` has an infinite width.", name);
        }
        if size.height.is_infinite() {
            let name = self.widget().type_name();
            warn!("Widget `{}` has an infinite height.", name);
        }
    }

    // --- PAINT ---

    /// Paint a child widget.
    ///
    /// Generally called by container widgets as part of their [`Widget::paint`]
    /// method.
    ///
    /// Note that this method does not apply the offset of the layout rect.
    /// If that is desired, use [`paint`] instead.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`Widget::paint`]: trait.Widget.html#tymethod.paint
    /// [`paint`]: #method.paint
    pub fn paint_raw(&mut self, ctx: &mut PaintCtx, env: &Env) {
        // we need to do this before we borrow from self
        if env.get(Env::DEBUG_WIDGET_ID) {
            self.make_widget_id_layout_if_needed(self.state.id, ctx, env);
        }

        self.call_widget_method_with_checks("paint", |widget_pod| {
            // widget_pod is a reborrow of `self`

            let mut inner_ctx = PaintCtx {
                render_ctx: ctx.render_ctx,
                global_state: ctx.global_state,
                z_ops: Vec::new(),
                region: ctx.region.clone(),
                widget_state: &widget_pod.state,
                depth: ctx.depth,
            };
            widget_pod.inner.paint(&mut inner_ctx, env);

            let debug_ids = inner_ctx.is_hot() && env.get(Env::DEBUG_WIDGET_ID);
            if debug_ids {
                // this also draws layout bounds
                widget_pod.debug_paint_widget_ids(&mut inner_ctx, env);
            }

            if !debug_ids && env.get(Env::DEBUG_PAINT) {
                widget_pod.debug_paint_layout_bounds(&mut inner_ctx, env);
            }

            ctx.z_ops.append(&mut inner_ctx.z_ops);
        });
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    ///
    /// This will recursively paint widgets, stopping if a widget's layout
    /// rect is outside of the currently visible region.
    pub fn paint(&mut self, parent_ctx: &mut PaintCtx, env: &Env) {
        self.paint_impl(parent_ctx, env, false)
    }

    /// Paint the widget, even if its layout rect is outside of the currently
    /// visible region.
    pub fn paint_always(&mut self, parent_ctx: &mut PaintCtx, env: &Env) {
        self.paint_impl(parent_ctx, env, true)
    }

    /// Shared implementation that can skip drawing non-visible content.
    fn paint_impl(&mut self, parent_ctx: &mut PaintCtx, env: &Env, paint_if_not_visible: bool) {
        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("paint");

        if !paint_if_not_visible && !parent_ctx.region().intersects(self.state.paint_rect()) {
            return;
        }

        parent_ctx.with_save(|ctx| {
            let layout_origin = self.layout_rect().origin().to_vec2();
            ctx.transform(Affine::translate(layout_origin));
            let mut visible = ctx.region().clone();
            visible.intersect_with(self.state.paint_rect());
            visible -= layout_origin;
            ctx.with_child_ctx(visible, |ctx| self.paint_raw(ctx, env));
        });
    }

    fn make_widget_id_layout_if_needed(&mut self, id: WidgetId, ctx: &mut PaintCtx, env: &Env) {
        if self.debug_widget_text.needs_rebuild() {
            // switch text color based on background, this is meh and that's okay
            let border_color = env.get_debug_color(id.to_raw());
            let (r, g, b, _) = border_color.as_rgba8();
            let avg = (r as u32 + g as u32 + b as u32) / 3;
            let text_color = if avg < 128 {
                Color::WHITE
            } else {
                Color::BLACK
            };
            let id_string = id.to_raw().to_string();
            self.debug_widget_text.set_text(id_string.into());
            self.debug_widget_text.set_text_size(10.0);
            self.debug_widget_text.set_text_color(text_color);
            self.debug_widget_text.rebuild_if_needed(ctx.text(), env);
        }
    }

    fn debug_paint_widget_ids(&self, ctx: &mut PaintCtx, env: &Env) {
        // we clone because we need to move it for paint_with_z_index
        let text = self.debug_widget_text.clone();
        let text_size = text.size();
        let origin = ctx.size().to_vec2() - text_size.to_vec2();
        let border_color = env.get_debug_color(ctx.widget_id().to_raw());
        self.debug_paint_layout_bounds(ctx, env);

        ctx.paint_with_z_index(ctx.depth(), move |ctx| {
            let origin = Point::new(origin.x.max(0.0), origin.y.max(0.0));
            let text_rect = Rect::from_origin_size(origin, text_size);
            ctx.fill(text_rect, &border_color);
            text.draw(ctx, origin);
        })
    }

    fn debug_paint_layout_bounds(&self, ctx: &mut PaintCtx, env: &Env) {
        const BORDER_WIDTH: f64 = 1.0;
        let rect = ctx.size().to_rect().inset(BORDER_WIDTH / -2.0);
        let id = self.id().to_raw();
        let color = env.get_debug_color(id);
        ctx.stroke(rect, &color, BORDER_WIDTH);
    }
}

impl<W: Widget> AsWidgetPod for WidgetPod<W> {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    // FIXME - remove
    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }

    // Return a reference to the inner widget.
    fn widget(&self) -> &dyn Widget {
        &self.inner
    }

    // FIXME - remove
    fn widget_mut(&mut self) -> &mut dyn Widget {
        &mut self.inner
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        self.inner.children()
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        self.inner.children_mut()
    }

    fn find_widget_by_id(&self, id: WidgetId) -> Option<&dyn AsWidgetPod> {
        WidgetPod::find_widget_by_id(self, id)
    }

    fn find_widget_at_pos(&self, pos: Point) -> Option<&dyn AsWidgetPod> {
        WidgetPod::find_widget_at_pos(self, pos)
    }
}

// ---

#[cfg(FALSE)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext_event::ExtEventHost;
    use crate::text::ParseFormatter;
    use crate::widget::{Flex, Scroll, Split, TextBox};
    use crate::{WidgetExt, WindowHandle, WindowId};
    use test_env_log::test;

    const ID_1: WidgetId = WidgetId::reserved(0);
    const ID_2: WidgetId = WidgetId::reserved(1);
    const ID_3: WidgetId = WidgetId::reserved(2);

    #[test]
    fn register_children() {
        fn make_widgets() -> impl Widget<u32> {
            Split::columns(
                Flex::<u32>::row()
                    .with_child(
                        TextBox::new()
                            .with_formatter(ParseFormatter::new())
                            .with_id(ID_1),
                    )
                    .with_child(
                        TextBox::new()
                            .with_formatter(ParseFormatter::new())
                            .with_id(ID_2),
                    )
                    .with_child(
                        TextBox::new()
                            .with_formatter(ParseFormatter::new())
                            .with_id(ID_3),
                    ),
                Scroll::new(TextBox::new().with_formatter(ParseFormatter::new())),
            )
        }

        let widget = make_widgets();
        let mut widget = WidgetPod::new(widget).boxed();

        let mut widget_state = WidgetState::new(WidgetId::next(), None);
        let window = WindowHandle::default();
        let ext_host = ExtEventHost::default();
        let ext_handle = ext_host.make_sink();
        let mut state = ContextState::new::<Option<u32>>(
            &mut command_queue,
            &ext_handle,
            &window,
            WindowId::next(),
            None,
        );

        let mut ctx = LifeCycleCtx {
            widget_state: &mut widget_state,
            global_state: &mut global_state,
        };

        let env = Env::with_default_i10n();

        widget.lifecycle(&mut ctx, &LifeCycle::WidgetAdded, &1, &env);
        assert!(ctx.widget_state.children.may_contain(&ID_1));
        assert!(ctx.widget_state.children.may_contain(&ID_2));
        assert!(ctx.widget_state.children.may_contain(&ID_3));
        // A textbox is composed of three components with distinct ids
        assert_eq!(ctx.widget_state.children.entry_count(), 15);
    }
}
