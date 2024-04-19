// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use tracing::{info_span, trace, warn};
use vello::Scene;
use winit::dpi::PhysicalPosition;

use crate::event::{PointerEvent, TextEvent};
use crate::kurbo::{Affine, Insets, Point, Rect, Shape, Size};
use crate::paint_scene_helpers::stroke;
use crate::render_root::RenderRootState;
use crate::theme::get_debug_color;
use crate::widget::{FocusChange, WidgetRef, WidgetState};
use crate::{
    BoxConstraints, EventCtx, InternalLifeCycle, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    StatusChange, Widget, WidgetId,
};

// TODO - rewrite links in doc

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// `WidgetPod` will translate internal Masonry events to regular events,
/// synthesize additional events of interest, and stop propagation when it makes sense.
pub struct WidgetPod<W> {
    pub(crate) state: WidgetState,
    pub(crate) inner: W,
    pub(crate) fragment: Scene,
}

// ---

impl<W: Widget> WidgetPod<W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<W> {
        Self::new_with_id(inner, WidgetId::next())
    }

    /// Create a new widget pod with fixed id.
    pub fn new_with_id(inner: W, id: WidgetId) -> WidgetPod<W> {
        let mut state = WidgetState::new(id, None, inner.short_type_name());
        state.children_changed = true;
        state.needs_layout = true;
        WidgetPod {
            state,
            inner,
            fragment: Scene::new(),
        }
    }

    /// Read-only access to state. We don't mark the field as `pub` because
    /// we want to control mutation.
    pub(crate) fn state(&self) -> &WidgetState {
        &self.state
    }

    // TODO - remove
    /// Return a reference to the inner widget.
    pub fn widget(&self) -> &W {
        &self.inner
    }

    /// Return a [`WidgetRef`] to the inner widget.
    pub fn as_ref(&self) -> WidgetRef<'_, W> {
        WidgetRef::new(&self.state, &self.inner)
    }

    /// Return a type-erased [`WidgetRef`] to the inner widget.
    pub fn as_dyn(&self) -> WidgetRef<'_, dyn Widget> {
        WidgetRef::new(&self.state, &self.inner)
    }

    /// Return `true` if the widget has received [`LifeCycle::WidgetAdded`].
    ///
    /// [`LifeCycle::WidgetAdded`]: ./enum.LifeCycle.html#variant.WidgetAdded
    pub fn is_initialized(&self) -> bool {
        !self.state.is_new
    }

    /// Return `true` if widget or any descendent is focused
    pub fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    /// Query the "active" state of the widget.
    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    /// Return `true` if any descendant is active.
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

    /// Return the layout rectangle.
    ///
    /// This will be a [`Rect`] with a [`Size`] determined by the child's [`layout`]
    /// method, and the origin that was set by [`place_child`].
    ///
    /// Two sibling widgets' layout rects will almost never intersect.
    ///
    /// This rect wil also be used to detect whether any given pointer event (eg clicks)
    /// intersects with the rectangle.
    ///
    /// [`Rect`]: struct.Rect.html
    /// [`Size`]: struct.Size.html
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`place_child`]: LayoutCtx::place_child
    pub fn layout_rect(&self) -> Rect {
        self.state.layout_rect()
    }

    /// Get the widget's paint rectangle.
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

    // FIXME - Remove
    /// Return a mutable reference to the inner widget.
    pub(crate) fn widget_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W: Widget> WidgetPod<W> {
    // TODO - this is confusing
    #[inline(always)]
    pub(crate) fn mark_as_visited(&mut self) {
        self.state.mark_as_visited(true);
    }

    // Notes about hot state:
    //
    // Hot state (the thing that changes when your mouse hovers over a button) is annoying to implement, because it breaks the convenient abstraction of multiple static passes over the widget tree.
    //
    // Ideally, what you'd want is "first handle events, then update widget states, then compute layout, then paint", where each 'then' is an indestructible wall that only be crossed in one direction.
    //
    // Hot state breaks that abstraction, because a change in a widget's layout (eg a button gets bigger) can lead to a change in hot state.
    //
    // To give an extreme example: suppose you have a button which becomes very small when you hover over it (and forget all the reasons this would be terrible UX). How should its hot state be handled? When the mouse moves over the button, the hot state will get changed, and the button will become smaller. But becoming smaller make it so the mouse no longer hovers over the button, so the hot state will get changed again.
    //
    // Ideally, this is a UX trap I'd like to warn against; in any case, the fact that it's possible shows we have to account for cases where layout has an influence on previous stages.
    //
    // In actual Masonry code, that means:
    // - `Widget::lifecycle` can be called within `Widget::layout`.
    // - `Widget::set_position` can call `Widget::lifecycle` and thus needs to be passed context types, which gives the method a surprising prototype.
    //
    // We could have `set_position` set a `hot_state_needs_update` flag, but then we'd need to add in another UpdateHotState pass (probably as a variant to the Lifecycle enum).
    //
    // Another problem is that hot state handling is counter-intuitive for someone writing a Widget implementation. Developers who want to implement "This widget turns red when the mouse is over it" will usually assume they should use the MouseMove event or something similar; when what they actually need is a Lifecycle variant.
    //
    // Other things hot state is missing:
    // - A concept of "cursor moved to inner widget" (though I think's that's not super useful outside the browser).
    // - Multiple pointers handling.

    /// Determines if the provided `mouse_pos` is inside `rect`
    /// and if so updates the hot state and sends `LifeCycle::HotChanged`.
    ///
    /// Return `true` if the hot state changed.
    ///
    /// The provided `child_state` should be merged up if this returns `true`.
    pub(crate) fn update_hot_state(
        inner: &mut W,
        inner_state: &mut WidgetState,
        global_state: &mut RenderRootState,
        mouse_pos: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let rect = inner_state.layout_rect() + inner_state.parent_window_origin.to_vec2();
        let had_hot = inner_state.is_hot;
        inner_state.is_hot = match mouse_pos {
            Some(pos) => rect.winding(Point::new(pos.x, pos.y)) != 0,
            None => false,
        };
        // FIXME - don't send event, update flags instead
        if had_hot != inner_state.is_hot {
            trace!(
                "Widget '{}' #{}: set hot state to {}",
                inner.short_type_name(),
                inner_state.id.to_raw(),
                inner_state.is_hot
            );

            let hot_changed_event = StatusChange::HotChanged(inner_state.is_hot);
            let mut inner_ctx = LifeCycleCtx {
                global_state,
                widget_state: inner_state,
            };

            let _span = info_span!("on_status_change").entered();
            inner.on_status_change(&mut inner_ctx, &hot_changed_event);

            return true;
        }
        false
    }

    // TODO - document
    // TODO - This method should take a 'can_skip: Fn(WidgetRef) -> bool'
    // predicate and only panic if can_skip returns false.
    #[inline(always)]
    fn call_widget_method_with_checks<Ret>(
        &mut self,
        method_name: &str,
        visit: impl FnOnce(&mut Self) -> Ret,
    ) -> Ret {
        if cfg!(not(debug_assertions)) {
            return visit(self);
        }

        for child in self.inner.children() {
            child.state().mark_as_visited(false);
        }
        let children_ids: Vec<_> = self.inner.children().iter().map(|w| w.id()).collect();

        let return_value = visit(self);

        let new_children_ids: Vec<_> = self.inner.children().iter().map(|w| w.id()).collect();
        if children_ids != new_children_ids && !self.state.children_changed {
            debug_panic!(
                "Error in '{}' #{}: children changed in method {} but ctx.children_changed() wasn't called",
                self.inner.short_type_name(),
                self.state().id.to_raw(),
                method_name,
            )
        }

        #[cfg(debug_assertions)]
        for child in self.inner.children() {
            // FIXME - use can_skip callback instead
            if child.state().needs_visit() && !child.state().is_stashed {
                debug_panic!(
                    "Error in '{}' #{}: child widget '{}' #{} not visited in method {}",
                    self.inner.short_type_name(),
                    self.state().id.to_raw(),
                    child.deref().short_type_name(),
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
        WidgetPod::new_with_id(Box::new(self.inner), self.state.id)
    }
}

// --- TRAIT IMPLS ---

impl<W: Widget> WidgetPod<W> {
    /// --- ON_EVENT ---

    // TODO #5 - Some implicit invariants:
    // - If a Widget gets a keyboard event or an ImeStateChange, then
    // focus is on it, its child or its parent.
    // - If a Widget has focus, then none of its parents is hidden

    pub fn on_pointer_event(&mut self, parent_ctx: &mut EventCtx, event: &PointerEvent) {
        let _span = self.inner.make_trace_span().entered();
        // TODO #11
        parent_ctx
            .global_state
            .debug_logger
            .push_span(self.inner.short_type_name());

        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("on_pointer_event");

        trace!(
            "Widget '{}' #{} visited",
            self.inner.short_type_name(),
            self.state.id.to_raw(),
        );

        if parent_ctx.is_handled {
            parent_ctx.global_state.debug_logger.pop_span();
            // If the event was already handled, we quit early.
            return;
        }

        let had_active = self.state.has_active;

        // TODO - This doesn't handle the case where multiple cursors
        // are over the same widget
        let hot_pos = match event {
            PointerEvent::PointerDown(_, pointer_state) => Some(pointer_state.position),
            PointerEvent::PointerUp(_, pointer_state) => Some(pointer_state.position),
            PointerEvent::PointerMove(pointer_state) => Some(pointer_state.position),
            PointerEvent::PointerEnter(pointer_state) => Some(pointer_state.position),
            PointerEvent::PointerLeave(_) => None,
            PointerEvent::MouseWheel(_, pointer_state) => Some(pointer_state.position),
            PointerEvent::HoverFile(_, _) => None,
            PointerEvent::DropFile(_, _) => None,
            PointerEvent::HoverFileCancel(_) => None,
        };
        let hot_changed = WidgetPod::update_hot_state(
            &mut self.inner,
            &mut self.state,
            parent_ctx.global_state,
            hot_pos,
        );
        let call_inner = (had_active || self.state.is_hot || hot_changed) && !self.state.is_stashed;
        //let call_inner = true;

        if call_inner {
            self.call_widget_method_with_checks("on_pointer_event", |widget_pod| {
                // widget_pod is a reborrow of `self`
                let mut inner_ctx = EventCtx {
                    global_state: parent_ctx.global_state,
                    widget_state: &mut widget_pod.state,
                    is_handled: false,
                    request_pan_to_child: None,
                };
                inner_ctx.widget_state.has_active = false;

                widget_pod.inner.on_pointer_event(&mut inner_ctx, event);

                inner_ctx.widget_state.has_active |= inner_ctx.widget_state.is_active;
                parent_ctx.is_handled |= inner_ctx.is_handled;

                // TODO - there's some dubious logic here
                if let Some(target_rect) = inner_ctx.request_pan_to_child {
                    widget_pod.pan_to_child(parent_ctx, target_rect);
                    let new_rect = target_rect
                        .with_origin(target_rect.origin() + widget_pod.state.origin.to_vec2());
                    parent_ctx.request_pan_to_child = Some(new_rect);
                }
            });
        }

        // Always merge even if not needed, because merging is idempotent and gives us simpler code.
        // Doing this conditionally only makes sense when there's a measurable performance boost.
        parent_ctx.widget_state.merge_up(&mut self.state);

        parent_ctx
            .global_state
            .debug_logger
            .update_widget_state(self.as_dyn());
        parent_ctx
            .global_state
            .debug_logger
            .push_log(false, "updated state");

        parent_ctx.global_state.debug_logger.pop_span();
    }

    pub fn on_text_event(&mut self, parent_ctx: &mut EventCtx, event: &TextEvent) {
        let _span = self.inner.make_trace_span().entered();
        // TODO #11
        parent_ctx
            .global_state
            .debug_logger
            .push_span(self.inner.short_type_name());

        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("on_text_event");

        if parent_ctx.is_handled {
            parent_ctx.global_state.debug_logger.pop_span();
            // If the event was already handled, we quit early.
            return;
        }

        if self.state.has_focus {
            self.call_widget_method_with_checks("on_text_event", |widget_pod| {
                // widget_pod is a reborrow of `self`
                let mut inner_ctx = EventCtx {
                    global_state: parent_ctx.global_state,
                    widget_state: &mut widget_pod.state,
                    is_handled: false,
                    request_pan_to_child: None,
                };

                widget_pod.inner.on_text_event(&mut inner_ctx, event);

                inner_ctx.widget_state.has_active |= inner_ctx.widget_state.is_active;
                parent_ctx.is_handled |= inner_ctx.is_handled;

                // TODO - there's some dubious logic here
                if let Some(target_rect) = inner_ctx.request_pan_to_child {
                    widget_pod.pan_to_child(parent_ctx, target_rect);
                    let new_rect = target_rect
                        .with_origin(target_rect.origin() + widget_pod.state.origin.to_vec2());
                    parent_ctx.request_pan_to_child = Some(new_rect);
                }
            });
        }

        // Always merge even if not needed, because merging is idempotent and gives us simpler code.
        // Doing this conditionally only makes sense when there's a measurable performance boost.
        parent_ctx.widget_state.merge_up(&mut self.state);

        parent_ctx
            .global_state
            .debug_logger
            .update_widget_state(self.as_dyn());
        parent_ctx
            .global_state
            .debug_logger
            .push_log(false, "updated state");

        parent_ctx.global_state.debug_logger.pop_span();
    }

    fn pan_to_child(&mut self, parent_ctx: &mut EventCtx, rect: Rect) {
        let mut inner_ctx = LifeCycleCtx {
            global_state: parent_ctx.global_state,
            widget_state: &mut self.state,
        };
        let event = LifeCycle::RequestPanToChild(rect);

        self.inner.lifecycle(&mut inner_ctx, &event);
    }

    // --- LIFECYCLE ---

    // TODO #5 - Some implicit invariants:
    // - A widget only receives BuildFocusChain if none of its parents are hidden.

    /// Propagate a [`LifeCycle`] event.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    pub fn lifecycle(&mut self, parent_ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        let _span = self.inner.make_trace_span().entered();

        // TODO #11
        parent_ctx
            .global_state
            .debug_logger
            .push_span(self.inner.short_type_name());

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
                        self.lifecycle(parent_ctx, &LifeCycle::WidgetAdded);
                        parent_ctx
                            .global_state
                            .debug_logger
                            .update_widget_state(self.as_dyn());
                        parent_ctx
                            .global_state
                            .debug_logger
                            .push_log(false, "updated state");
                        parent_ctx.global_state.debug_logger.pop_span();
                        return;
                    } else {
                        if self.state.children_changed {
                            // TODO - Separate "widget removed" case.
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
                        // TODO
                        let disabled = self.state.is_disabled();
                        self.call_widget_method_with_checks("lifecycle", |widget_pod| {
                            let mut inner_ctx = LifeCycleCtx {
                                global_state: parent_ctx.global_state,
                                widget_state: &mut widget_pod.state,
                            };

                            widget_pod
                                .inner
                                .lifecycle(&mut inner_ctx, &LifeCycle::DisabledChanged(disabled));
                        });
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
                    // TODO - self.state.is_hidden
                    true
                }
            },
            LifeCycle::WidgetAdded => {
                if !self.state.is_new {
                    // TODO - better warning.
                    warn!("Already initialized.");
                }
                trace!(
                    "{} Received LifeCycle::WidgetAdded",
                    self.inner.short_type_name()
                );

                self.state.is_new = false;
                self.state.update_focus_chain = true;
                self.state.needs_layout = true;
                self.state.needs_paint = true;

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
            LifeCycle::AnimFrame(_) => true,
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
            LifeCycle::BuildFocusChain => {
                if self.state.update_focus_chain {
                    // Replace has_focus to check if the value changed in the meantime
                    let is_focused = parent_ctx.global_state.focused_widget == Some(self.state.id);
                    self.state.has_focus = is_focused;

                    self.state.focus_chain.clear();
                    true
                } else {
                    false
                }
            }
            // This is called by children when going up the widget tree.
            LifeCycle::RequestPanToChild(_) => false,
        };

        // widget_pod is a reborrow of `self`
        if call_inner {
            self.call_widget_method_with_checks("lifecycle", |widget_pod| {
                let mut inner_ctx = LifeCycleCtx {
                    global_state: parent_ctx.global_state,
                    widget_state: &mut widget_pod.state,
                };

                widget_pod.inner.lifecycle(&mut inner_ctx, event);
            });
        }

        if let Some(event) = extra_event.as_ref() {
            let mut inner_ctx = LifeCycleCtx {
                global_state: parent_ctx.global_state,
                widget_state: &mut self.state,
            };

            // We add a span so that inner logs are marked as being in an on_status_change pass
            let _span = info_span!("on_status_change").entered();
            self.inner.on_status_change(&mut inner_ctx, event);
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
            LifeCycle::DisabledChanged(_)
            | LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged) => {
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

        parent_ctx
            .global_state
            .debug_logger
            .update_widget_state(self.as_dyn());
        parent_ctx
            .global_state
            .debug_logger
            .push_log(false, "updated state");

        parent_ctx.global_state.debug_logger.pop_span();
    }

    // --- LAYOUT ---

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn layout(&mut self, parent_ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let _span = self.inner.make_trace_span().entered();

        // TODO #11
        parent_ctx
            .global_state
            .debug_logger
            .push_span(self.inner.short_type_name());

        if self.state.is_stashed {
            debug_panic!(
                "Error in '{}' #{}: trying to compute layout of stashed widget.",
                self.inner.short_type_name(),
                self.state().id.to_raw(),
            );
            return Size::ZERO;
        }

        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("layout");

        self.state.needs_layout = false;
        self.state.needs_window_origin = false;
        self.state.is_expecting_place_child_call = true;
        // TODO - Not everything that has been re-laid out needs to be repainted.
        self.state.needs_paint = true;

        bc.debug_check(self.inner.short_type_name());

        self.state.local_paint_rect = Rect::ZERO;

        let new_size = self.call_widget_method_with_checks("layout", |widget_pod| {
            // widget_pod is a reborrow of `self`

            let mut inner_ctx = LayoutCtx {
                widget_state: &mut widget_pod.state,
                global_state: parent_ctx.global_state,
                mouse_pos: parent_ctx.mouse_pos,
            };

            widget_pod.inner.layout(&mut inner_ctx, bc)
        });

        self.state.local_paint_rect = self
            .state
            .local_paint_rect
            .union(new_size.to_rect() + self.state.paint_insets);

        if cfg!(debug_assertions) {
            for child in self.inner.children() {
                if child.state().is_expecting_place_child_call {
                    debug_panic!(
                        "Error in '{}' #{}: missing call to place_child method for child widget '{}' #{}. During layout pass, if a widget calls WidgetPod::layout() on its child, it then needs to call LayoutCtx::place_child() on the same child.",
                        self.inner.short_type_name(),
                        self.state().id.to_raw(),
                        child.deref().short_type_name(),
                        child.state().id.to_raw(),
                    );
                }

                // TODO - This check might be redundant with the code updating local_paint_rect
                let child_rect = child.state().paint_rect();
                if !rect_contains(&self.state.local_paint_rect, &child_rect)
                    && !self.state.is_portal
                {
                    debug_panic!(
                        "Error in '{}' #{}: paint_rect {:?} doesn't contain paint_rect {:?} of child widget '{}' #{}",
                        self.inner.short_type_name(),
                        self.state().id.to_raw(),
                        self.state.local_paint_rect,
                        child_rect,
                        child.deref().short_type_name(),
                        child.state().id.to_raw(),
                    );
                }
            }
        }

        // TODO - Figure out how to deal with the overflow problem, eg:
        // What happens if a widget returns a size larger than the allowed constraints?
        // Some possibilities are:
        // - Always clip: might be expensive
        // - Display it anyway: might lead to graphical bugs
        // - Panic: too harsh?
        // Also, we need to avoid spurious crashes when we initialize the app and the
        // size is (0,0)
        // See issue #4

        parent_ctx.widget_state.merge_up(&mut self.state);
        self.state.size = new_size;
        self.log_layout_issues(new_size);

        parent_ctx
            .global_state
            .debug_logger
            .update_widget_state(self.as_dyn());
        parent_ctx
            .global_state
            .debug_logger
            .push_log(false, "updated state");

        parent_ctx.global_state.debug_logger.pop_span();

        new_size
    }

    fn log_layout_issues(&self, size: Size) {
        if size.width.is_infinite() {
            let name = self.inner.type_name();
            warn!("Widget `{}` has an infinite width.", name);
        }
        if size.height.is_infinite() {
            let name = self.inner.type_name();
            warn!("Widget `{}` has an infinite height.", name);
        }
    }

    // --- PAINT ---

    /// Paint the widget, translating it by the origin of its layout rectangle.
    ///
    /// This will recursively paint widgets, stopping if a widget's layout
    /// rect is outside of the currently visible region.
    pub fn paint(&mut self, parent_ctx: &mut PaintCtx, scene: &mut Scene) {
        let _span = self.inner.make_trace_span().entered();

        if self.state.is_stashed {
            debug_panic!(
                "Error in '{}' #{}: trying to paint stashed widget.",
                self.inner.short_type_name(),
                self.state().id.to_raw(),
            );
            return;
        }

        trace!(
            "Painting widget '{}' #{}",
            self.inner.short_type_name(),
            self.state.id.to_raw()
        );

        // TODO - explain this
        self.mark_as_visited();
        self.check_initialized("paint");

        if self.state.needs_paint {
            self.state.needs_paint = false;
            self.call_widget_method_with_checks("paint", |widget_pod| {
                // TODO - Handle invalidation regions
                let mut inner_ctx = PaintCtx {
                    global_state: parent_ctx.global_state,
                    widget_state: &widget_pod.state,
                    depth: parent_ctx.depth + 1,
                    debug_paint: parent_ctx.debug_paint,
                    debug_widget: parent_ctx.debug_widget,
                };

                widget_pod.fragment.reset();
                widget_pod
                    .inner
                    .paint(&mut inner_ctx, &mut widget_pod.fragment);

                if parent_ctx.debug_paint {
                    widget_pod.debug_paint_layout_bounds(widget_pod.state.size);
                }
            });
        }

        let transform = Affine::translate(self.state.origin.to_vec2());
        scene.append(&self.fragment, Some(transform));
    }

    fn debug_paint_layout_bounds(&mut self, size: Size) {
        const BORDER_WIDTH: f64 = 1.0;
        let rect = size.to_rect().inset(BORDER_WIDTH / -2.0);
        let id = self.id().to_raw();
        let color = get_debug_color(id);
        let scene = &mut self.fragment;
        stroke(scene, &rect, color, BORDER_WIDTH);
    }
}

// TODO - negative rects?
/// Return `true` if all of `smaller` is within `larger`.
fn rect_contains(larger: &Rect, smaller: &Rect) -> bool {
    smaller.x0 >= larger.x0
        && smaller.x1 <= larger.x1
        && smaller.y0 >= larger.y0
        && smaller.y1 <= larger.y1
}
