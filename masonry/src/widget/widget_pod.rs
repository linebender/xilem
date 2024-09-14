// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use smallvec::SmallVec;
use tracing::trace;

use crate::tree_arena::ArenaRefChildren;
use crate::widget::WidgetState;
use crate::{InternalLifeCycle, LifeCycle, LifeCycleCtx, Widget, WidgetId};

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
    pub(crate) fn call_widget_method_with_checks<Ctx>(
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
    // --- MARK: ON_XXX_EVENT ---

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

        let call_widget = match event {
            LifeCycle::Internal(internal) => match internal {
                InternalLifeCycle::RouteWidgetAdded => state.children_changed,
            },
            LifeCycle::WidgetAdded => {
                trace!(
                    "{} received LifeCycle::WidgetAdded",
                    widget.short_type_name()
                );

                true
            }
            // Routing DisabledChanged has been moved to the update_disabled pass
            LifeCycle::DisabledChanged(_) => false,
            // Animations have been moved to the update_anim pass
            LifeCycle::AnimFrame(_) => false,
            LifeCycle::BuildFocusChain => false,
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

        // Sync our state with our parent's state after the event!

        match event {
            // we need to (re)register children in case of one of the following events
            LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded) => {
                state.children_changed = false;
            }
            _ => (),
        }

        let state_mut = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        parent_ctx.widget_state.merge_up(state_mut.item);

        call_widget
    }
}
