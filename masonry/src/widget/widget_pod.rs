// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use smallvec::SmallVec;
use tracing::{info_span, trace, warn};
use vello::kurbo::{Rect, Size};

use crate::tree_arena::ArenaRefChildren;
use crate::widget::WidgetState;
use crate::{
    AccessCtx, BoxConstraints, InternalLifeCycle, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
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
    id: WidgetId,
    inner: WidgetPodInner<W>,
}

// TODO - This is a simple state machine that lets users create WidgetPods
// without immediate access to the widget arena. It's *extremely* inefficient
// and leads to ugly code. The alternative is to force users to create WidgetPods
// through context methods where they already have access to the arena.
// Implementing that requires solving non-trivial design questions.

enum WidgetPodInner<W> {
    Created(W),
    Inserted,
}

// --- MARK: GETTERS ---
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
        WidgetPod {
            id,
            inner: WidgetPodInner::Created(inner),
        }
    }

    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}

impl<W: Widget + 'static> WidgetPod<W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> WidgetPod<Box<dyn Widget>> {
        match self.inner {
            WidgetPodInner::Created(inner) => WidgetPod::new_with_id(Box::new(inner), self.id),
            WidgetPodInner::Inserted => {
                panic!("Cannot box a widget after it has been inserted into the widget graph")
            }
        }
    }
}

// --- MARK: INTERNALS ---
impl<W: Widget> WidgetPod<W> {
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
    // - A concept of "cursor moved to inner widget" (though I think that's not super useful outside the browser).
    // - Multiple pointers handling.

    // TODO - document
    // TODO - This method should take a 'can_skip: Fn(WidgetRef) -> bool'
    // predicate and only panic if can_skip returns false.
    #[inline(always)]
    fn call_widget_method_with_checks<Ctx>(
        &mut self,
        method_name: &str,
        ctx: &mut Ctx,
        get_tokens: impl Fn(
            &mut Ctx,
        ) -> (
            ArenaRefChildren<'_, WidgetState>,
            ArenaRefChildren<'_, Box<dyn Widget>>,
        ),
        visit: impl FnOnce(&mut Self, &mut Ctx) -> bool,
    ) {
        if let WidgetPodInner::Created(widget) = &self.inner {
            debug_panic!(
                "Error in '{}' #{}: method '{}' called before receiving WidgetAdded.",
                widget.short_type_name(),
                self.id().to_raw(),
                method_name,
            );
        }

        let id = self.id().to_raw();
        let (parent_state_mut, parent_token) = get_tokens(ctx);
        let widget_ref = parent_token
            .get_child(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let state_ref = parent_state_mut
            .get_child(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let widget = widget_ref.item;
        let state = state_ref.item;

        let _span = widget.make_trace_span().entered();

        // TODO https://github.com/linebender/xilem/issues/370 - Re-implement debug logger

        // TODO - explain this
        state.mark_as_visited(true);

        let mut children_ids = SmallVec::new();

        if cfg!(debug_assertions) {
            for child_state_ref in state_ref.children.iter_children() {
                child_state_ref.item.mark_as_visited(false);
            }
            children_ids = widget.children_ids();
        }

        let called_widget = visit(self, ctx);

        let (parent_state_mut, parent_token) = get_tokens(ctx);
        let widget_ref = parent_token
            .get_child(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let state_ref = parent_state_mut
            .get_child(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let widget = widget_ref.item;
        let state = state_ref.item;

        if cfg!(debug_assertions) && called_widget {
            let new_children_ids = widget.children_ids();
            if children_ids != new_children_ids && !state.children_changed {
                debug_panic!(
                    "Error in '{}' #{}: children changed in method {} but ctx.children_changed() wasn't called",
                    widget.short_type_name(),
                    self.id().to_raw(),
                    method_name,
                );
            }

            for id in &new_children_ids {
                let id = id.to_raw();
                if !state_ref.children.has_child(id) {
                    debug_panic!(
                        "Error in '{}' #{}: child widget #{} not added in method {}",
                        widget.short_type_name(),
                        self.id().to_raw(),
                        id,
                        method_name,
                    );
                }
            }

            #[cfg(debug_assertions)]
            for child_state_ref in state_ref.children.iter_children() {
                // FIXME - use can_skip callback instead
                if child_state_ref.item.needs_visit() && !child_state_ref.item.is_stashed {
                    debug_panic!(
                        "Error in '{}' #{}: child widget '{}' #{} not visited in method {}",
                        widget.short_type_name(),
                        self.id().to_raw(),
                        child_state_ref.item.widget_name,
                        child_state_ref.item.id.to_raw(),
                        method_name,
                    );
                }
            }
        }
    }
}

impl<W: Widget> WidgetPod<W> {
    /// --- MARK: ON_XXX_EVENT ---

    // TODO https://github.com/linebender/xilem/issues/376 - Some implicit invariants:
    // - If a Widget gets a keyboard event or an ImeStateChange, then
    // focus is on it, its child or its parent.
    // - If a Widget has focus, then none of its parents is hidden

    // --- MARK: LIFECYCLE ---

    // TODO https://github.com/linebender/xilem/issues/376 - Some implicit invariants:
    // - A widget only receives BuildFocusChain if none of its parents are hidden.

    /// Propagate a [`LifeCycle`] event.
    pub fn lifecycle(&mut self, parent_ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        if matches!(self.inner, WidgetPodInner::Created(_)) {
            let early_return = self.lifecycle_inner_added(parent_ctx, event);
            if early_return {
                return;
            }
        }
        self.call_widget_method_with_checks(
            "lifecycle",
            parent_ctx,
            |ctx| {
                (
                    ctx.widget_state_children.reborrow(),
                    ctx.widget_children.reborrow(),
                )
            },
            |self2, parent_ctx| self2.lifecycle_inner(parent_ctx, event),
        );
    }

    // This handles the RouteWidgetAdded cases
    fn lifecycle_inner_added(&mut self, parent_ctx: &mut LifeCycleCtx, event: &LifeCycle) -> bool {
        // Note: this code is the absolute worse and needs to die in a fire.
        // We're basically implementing a system where RouteWidgetAdded is
        // propagated to a bunch of widgets, and transformed into WidgetAdded,
        // which is *also* propagated to children but we want to skip that case.
        match event {
            LifeCycle::WidgetAdded => {
                return true;
            }
            _ => (),
        }

        let widget = match std::mem::replace(&mut self.inner, WidgetPodInner::Inserted) {
            WidgetPodInner::Created(widget) => widget,
            WidgetPodInner::Inserted => unreachable!(),
        };
        let id = self.id().to_raw();

        let _span = widget.make_trace_span().entered();

        match event {
            LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded) => {}
            event => {
                debug_panic!(
                    "Error in '{}' #{id}: method 'lifecycle' called with {event:?} before receiving WidgetAdded.",
                    widget.short_type_name(),
                );
            }
        }

        let state = WidgetState::new(self.id, widget.short_type_name());

        parent_ctx
            .widget_children
            .insert_child(id, Box::new(widget));
        parent_ctx.widget_state_children.insert_child(id, state);

        self.lifecycle_inner(parent_ctx, &LifeCycle::WidgetAdded);
        false
    }

    fn lifecycle_inner(&mut self, parent_ctx: &mut LifeCycleCtx, event: &LifeCycle) -> bool {
        let id = self.id().to_raw();
        let mut widget_mut = parent_ctx
            .widget_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let mut state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let widget = widget_mut.item;
        let state = state_mut.item;

        // when routing a status change event, if we are at our target
        // we may send an extra event after the actual event
        let mut extra_event = None;

        let had_focus = state.has_focus;

        let call_widget = match event {
            LifeCycle::Internal(internal) => match internal {
                InternalLifeCycle::RouteWidgetAdded => {
                    // TODO - explain
                    if state.children_changed {
                        // TODO - Separate "widget removed" case.
                        state.children.clear();
                    }
                    state.children_changed
                }
                InternalLifeCycle::RouteDisabledChanged => {
                    state.update_focus_chain = true;

                    let was_disabled = state.is_disabled();

                    state.is_explicitly_disabled = state.is_explicitly_disabled_new;

                    if was_disabled != state.is_disabled() {
                        // TODO
                        let disabled = state.is_disabled();

                        let mut inner_ctx = LifeCycleCtx {
                            global_state: parent_ctx.global_state,
                            widget_state: state,
                            widget_state_children: state_mut.children.reborrow_mut(),
                            widget_children: widget_mut.children.reborrow_mut(),
                        };

                        widget.lifecycle(&mut inner_ctx, &LifeCycle::DisabledChanged(disabled));

                        //Each widget needs only one of DisabledChanged and RouteDisabledChanged
                        false
                    } else {
                        state.children_disabled_changed
                    }
                }
                InternalLifeCycle::RouteFocusChanged { old, new } => {
                    let this_changed = if *old == Some(self.id()) {
                        Some(false)
                    } else if *new == Some(self.id()) {
                        Some(true)
                    } else {
                        None
                    };

                    if let Some(change) = this_changed {
                        state.has_focus = change;
                        extra_event = Some(StatusChange::FocusChanged(change));
                    } else {
                        state.has_focus = false;
                    }

                    // Recurse when the target widgets could be our descendants.
                    // The bloom filter we're checking can return false positives.
                    match (old, new) {
                        (Some(old), _) if state.children.may_contain(old) => true,
                        (_, Some(new)) if state.children.may_contain(new) => true,
                        _ => false,
                    }
                }
            },
            LifeCycle::WidgetAdded => {
                trace!(
                    "{} received LifeCycle::WidgetAdded",
                    widget.short_type_name()
                );

                true
            }
            LifeCycle::AnimFrame(_) => {
                state.request_anim = false;
                true
            }
            LifeCycle::DisabledChanged(ancestors_disabled) => {
                state.update_focus_chain = true;

                let was_disabled = state.is_disabled();

                state.is_explicitly_disabled = state.is_explicitly_disabled_new;
                state.ancestor_disabled = *ancestors_disabled;

                // the change direction (true -> false or false -> true) of our parent and ourself
                // is always the same, or we dont change at all, because we stay disabled if either
                // we or our parent are disabled.
                was_disabled != state.is_disabled()
            }
            LifeCycle::BuildFocusChain => {
                if state.update_focus_chain {
                    // Replace has_focus to check if the value changed in the meantime
                    let is_focused = parent_ctx.global_state.focused_widget == Some(self.id());
                    state.has_focus = is_focused;

                    state.focus_chain.clear();
                    true
                } else {
                    false
                }
            }
            // This is called by children when going up the widget tree.
            LifeCycle::RequestPanToChild(_) => false,
        };

        if call_widget {
            let mut inner_ctx = LifeCycleCtx {
                global_state: parent_ctx.global_state,
                widget_state: state,
                widget_state_children: state_mut.children.reborrow_mut(),
                widget_children: widget_mut.children.reborrow_mut(),
            };

            widget.lifecycle(&mut inner_ctx, event);
        }

        if let Some(event) = extra_event.as_ref() {
            let mut inner_ctx = LifeCycleCtx {
                global_state: parent_ctx.global_state,
                widget_state: state,
                widget_state_children: state_mut.children.reborrow_mut(),
                widget_children: widget_mut.children.reborrow_mut(),
            };

            // We add a span so that inner logs are marked as being in an on_status_change pass
            let _span = info_span!("on_status_change").entered();
            widget.on_status_change(&mut inner_ctx, event);
        }

        // Sync our state with our parent's state after the event!

        match event {
            // we need to (re)register children in case of one of the following events
            LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded) => {
                state.children_changed = false;
                parent_ctx.widget_state.children =
                    parent_ctx.widget_state.children.union(state.children);
                parent_ctx.register_child(self.id());
            }
            LifeCycle::DisabledChanged(_)
            | LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged) => {
                state.children_disabled_changed = false;

                if state.is_disabled() && state.has_focus {
                    // This may gets overwritten. This is ok because it still ensures that a
                    // FocusChange is routed after we updated the focus-chain.
                    parent_ctx.global_state.next_focused_widget = None;
                }

                // Delete changes of disabled state that happened during DisabledChanged to avoid
                // recursions.
                state.is_explicitly_disabled_new = state.is_explicitly_disabled;
            }
            // Update focus-chain of our parent
            LifeCycle::BuildFocusChain => {
                state.update_focus_chain = false;

                // had_focus is the old focus value. state.has_focus was replaced with parent_ctx.is_focused().
                // Therefore if had_focus is true but state.has_focus is false then the widget which is
                // currently focused is not part of the functional tree anymore
                // (Lifecycle::BuildFocusChain.should_propagate_to_hidden() is false!) and should
                // resign the focus.
                if had_focus && !state.has_focus {
                    // Not sure about this logic, might remove
                    parent_ctx.global_state.next_focused_widget = None;
                }
                state.has_focus = had_focus;

                if !state.is_disabled() {
                    parent_ctx
                        .widget_state
                        .focus_chain
                        .extend(&state.focus_chain);
                }
            }
            _ => (),
        }

        let state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        parent_ctx.widget_state.merge_up(state_mut.item);

        call_widget || extra_event.is_some()
    }

    // --- MARK: LAYOUT ---

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: Widget::layout
    pub fn layout(&mut self, parent_ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        self.call_widget_method_with_checks(
            "layout",
            parent_ctx,
            |ctx| {
                (
                    ctx.widget_state_children.reborrow(),
                    ctx.widget_children.reborrow(),
                )
            },
            |self2, parent_ctx| self2.layout_inner(parent_ctx, bc),
        );

        let id = self.id().to_raw();
        let state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        state_mut.item.size
    }

    fn layout_inner(&mut self, parent_ctx: &mut LayoutCtx, bc: &BoxConstraints) -> bool {
        let id = self.id().to_raw();
        let widget_mut = parent_ctx
            .widget_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let mut state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let widget = widget_mut.item;
        let state = state_mut.item;

        if state.is_stashed {
            debug_panic!(
                "Error in '{}' #{}: trying to compute layout of stashed widget.",
                widget.short_type_name(),
                id,
            );
            state.size = Size::ZERO;
            return false;
        }

        bc.debug_check(widget.short_type_name());
        trace!("Computing layout with constraints {:?}", bc);

        state.local_paint_rect = Rect::ZERO;

        state.request_layout = false;
        let new_size = {
            let mut inner_ctx = LayoutCtx {
                widget_state: state,
                widget_state_children: state_mut.children.reborrow_mut(),
                widget_children: widget_mut.children,
                global_state: parent_ctx.global_state,
                mouse_pos: parent_ctx.mouse_pos,
            };

            widget.layout(&mut inner_ctx, bc)
        };

        if state.request_layout {
            debug_panic!(
                "Error in '{}' #{}: layout pass has requested layout.",
                widget.short_type_name(),
                id,
            );
        }

        // We reset `needs_layout` after the layout call, which means if
        // the widget sets needs_layout to true, that will be overridden.
        // This shouldn't happen in practice, except in one case: if we access
        // a child using `get_raw_mut` before the child's layout is run. In that
        // case the child's needs_layout is still true, and propagates up to
        // this widget. The line below resets it to false.
        state.needs_layout = false;

        state.needs_compose = true;
        state.is_expecting_place_child_call = true;
        // TODO - Not everything that has been re-laid out needs to be repainted.
        state.request_paint = true;
        state.needs_paint = true;
        state.request_accessibility = true;
        state.needs_accessibility = true;

        state.local_paint_rect = state
            .local_paint_rect
            .union(new_size.to_rect() + state.paint_insets);

        #[cfg(debug_assertions)]
        {
            for child_id in widget.children_ids() {
                let child_id = child_id.to_raw();
                let child_state_mut = state_mut
                    .children
                    .get_child_mut(child_id)
                    .unwrap_or_else(|| panic!("widget #{child_id} not found"));
                let child_state = child_state_mut.item;
                if child_state.is_expecting_place_child_call {
                    debug_panic!(
                        "Error in '{}' #{}: missing call to place_child method for child widget '{}' #{}. During layout pass, if a widget calls WidgetPod::layout() on its child, it then needs to call LayoutCtx::place_child() on the same child.",
                        widget.short_type_name(),
                        id,
                        child_state.widget_name,
                        child_state.id.to_raw(),
                    );
                }

                // TODO - This check might be redundant with the code updating local_paint_rect
                let child_rect = child_state.paint_rect();
                if !rect_contains(&state.local_paint_rect, &child_rect) && !state.is_portal {
                    debug_panic!(
                        "Error in '{}' #{}: paint_rect {:?} doesn't contain paint_rect {:?} of child widget '{}' #{}",
                        widget.short_type_name(),
                        id,
                        state.local_paint_rect,
                        child_rect,
                        child_state.widget_name,
                        child_state.id.to_raw(),
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
        // See https://github.com/linebender/xilem/issues/377

        let state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        parent_ctx.widget_state.merge_up(state_mut.item);
        state_mut.item.size = new_size;

        self.log_layout_issues(widget.short_type_name(), new_size);

        true
    }

    fn log_layout_issues(&self, type_name: &str, size: Size) {
        if size.width.is_infinite() {
            warn!("Widget `{type_name}` has an infinite width.");
        }
        if size.height.is_infinite() {
            warn!("Widget `{type_name}` has an infinite height.");
        }
    }

    // --- MARK: PAINT ---

    // TODO - This should be removed in a follow-up PR immediately after
    // this is merged. I'm leaving the method for now to avoid blowing up the diff.
    pub fn paint(&mut self, _parent_ctx: &mut PaintCtx, _scene: &mut vello::Scene) {}

    // --- MARK: ACCESSIBILITY ---

    // TODO - This should be removed in a follow-up PR immediately after
    // this is merged. I'm leaving the method for now to avoid blowing up the diff.
    pub fn accessibility(&mut self, _parent_ctx: &mut AccessCtx) {}
}

// TODO - negative rects?
/// Return `true` if all of `smaller` is within `larger`.
fn rect_contains(larger: &Rect, smaller: &Rect) -> bool {
    smaller.x0 >= larger.x0
        && smaller.x1 <= larger.x1
        && smaller.y0 >= larger.y0
        && smaller.y1 <= larger.y1
}
