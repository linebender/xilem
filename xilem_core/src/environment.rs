// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "Development")]
//! Values accessible throughout the Xilem view tree.

use core::{any::TypeId, marker::PhantomData};

use crate::{AnyMessage, MessageResult, View, ViewId, ViewMarker, ViewPathTracker};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use hashbrown::{HashMap, hash_map::Entry};

#[derive(Debug)]
pub struct Rebuild;

// --- MARK: Environment

#[derive(Debug)]
struct EnvironmentItem {
    value: Box<dyn AnyMessage>,
    // TODO: Can we/do we want to make these share an allocation?
    // TODO: How do we GC this?
    change_listeners: Vec<Option<Arc<[ViewId]>>>,
}

#[derive(Debug)]
struct Slot {
    item: Option<EnvironmentItem>,
    ref_count: u32,
    // generation: u32,
}

#[derive(Debug)]
pub struct Environment {
    slots: Vec<Slot>,
    // We use u32 here so that we could move to a generation
    free_slots: Vec<u32>,
    types: HashMap<TypeId, u32>,
    // TODO: Think about how to handle this.
    // queued_rebuilds: Vec<Arc<[ViewId]>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            types: HashMap::new(),
            // queued_rebuilds: Vec::new(),
        }
    }

    // TODO: Better generic name here.
    fn create_slot_for_type<Context>(&mut self) -> u32
    where
        Context: Resource,
    {
        match self.types.entry(TypeId::of::<Context>()) {
            Entry::Occupied(occupied_entry) => *occupied_entry.get(),
            Entry::Vacant(vacant_entry) => {
                if let Some(slot) = self.free_slots.pop() {
                    debug_assert_eq!(
                        self.slots[usize::try_from(slot).unwrap()].ref_count,
                        0,
                        "Free slot should actually be set and unused"
                    );
                    vacant_entry.insert(slot);
                    slot
                } else {
                    let slot: u32 = self
                        .slots
                        .len()
                        .try_into()
                        .expect("Should be fewer than 2.pow(32) resources/locals used.");
                    self.slots.push(Slot {
                        item: None,
                        ref_count: 0,
                    });
                    vacant_entry.insert(slot);
                    slot
                }
            }
        }
    }

    fn get_slot_for_type<Context>(&mut self) -> Option<u32>
    where
        Context: Resource,
    {
        self.types.get(&TypeId::of::<Context>()).copied()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker trait for types usable as resources.
pub trait Resource: AnyMessage {}

// --- MARK: Provides

pub fn provides<State, Action, Context, InitialContext, ChildView, Ctx>(
    initial_context: InitialContext,
    child: ChildView,
) -> Provides<State, Action, Context, InitialContext, ChildView>
where
    InitialContext: Fn(&mut State) -> Context,
    ChildView: View<State, Action, Ctx>,
    Ctx: ViewPathTracker,
    Context: Resource,
{
    Provides {
        initial_context,
        child,
        phantom: PhantomData,
    }
}

// TODO: This `Debug` impl is pretty stupid
#[derive(Debug)]
pub struct Provides<State, Action, Context: Resource, InitialContext, ChildView> {
    initial_context: InitialContext,
    child: ChildView,
    phantom: PhantomData<fn(State, Context) -> Action>,
}

// TODO: This type shouldn't be public
#[derive(Debug)]
pub struct ProvidesState<ChildState> {
    child_state: ChildState,
    this_state: Option<EnvironmentItem>,
    environment_slot: u32,
}

impl<State, Action, Context, InitialContext, ChildView> ViewMarker
    for Provides<State, Action, Context, InitialContext, ChildView>
where
    Context: Resource,
{
}

impl<State, Action, Context, InitialContext, Ctx: ViewPathTracker, ChildView>
    View<State, Action, Ctx> for Provides<State, Action, Context, InitialContext, ChildView>
where
    InitialContext: Fn(&mut State) -> Context,
    ChildView: View<State, Action, Ctx>,
    Context: Resource,
    Self: 'static,
{
    type Element = ChildView::Element;

    type ViewState = ProvidesState<ChildView::ViewState>;

    fn build(&self, ctx: &mut Ctx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // Prepare the initial state value
        let value = (self.initial_context)(app_state);
        let environment_item = EnvironmentItem {
            change_listeners: Vec::new(),
            value: Box::new(value),
        };

        let env = ctx.environment();
        let pos = env.create_slot_for_type::<Context>();

        // Run the child build with the context value we're providing in the slot.
        let slot_idx = usize::try_from(pos).unwrap();
        let slot = &mut env.slots[slot_idx];
        slot.ref_count += 1;
        let old_value = slot.item.replace(environment_item);
        #[cfg(debug_assertions)]
        if let Some(old_value) = old_value.as_ref() {
            assert!(
                old_value.value.is::<Context>(),
                "In providing {}, the type of the old value didn't match. The old value was instead {:?}",
                core::any::type_name::<Context>(),
                old_value.value
            );
        }
        let (child_element, child_state) = self.child.build(ctx, app_state);

        // Restore the prior value into the environment
        let env = ctx.environment();
        let slot = &mut env.slots[slot_idx];
        let my_item = core::mem::replace(&mut slot.item, old_value);
        let my_item =
            my_item.expect("Child Views should not have deleted the environment item's value.");
        debug_assert!(
            my_item.value.is::<Context>(),
            "Running a child build should have restored the same value"
        );

        let state = ProvidesState {
            child_state,
            this_state: Some(my_item),
            environment_slot: pos,
        };
        (child_element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        // Use our value in the child rebuild.
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should be providing something."
        );
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        self.child.rebuild(
            &prev.child,
            &mut view_state.child_state,
            ctx,
            element,
            app_state,
        );

        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should get its value back."
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        // Make our value available in the child teardown.
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        self.child
            .teardown(&mut view_state.child_state, ctx, element, app_state);

        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        slot.ref_count -= 1;
        if slot.ref_count == 0 {
            assert!(
                slot.item.is_none(),
                "Ref count for {slot:?} was not properly managed."
            );
            env.free_slots.push(view_state.environment_slot);
            env.types.remove(&TypeId::of::<Context>());
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        // TODO: Any need for a message directly to this view?
        // TODO: When the context/environment is available in messages, add the context value here.
        self.child
            .message(&mut view_state.child_state, id_path, message, app_state)
    }
}

// --- MARK: WithContext
pub fn with_context<State, Action, Context, Child, ChildView, Ctx>(
    child: Child,
) -> WithContext<State, Action, Context, Child, ChildView>
where
    ChildView: View<State, Action, Ctx>,
    Ctx: ViewPathTracker,
    Context: Resource,
{
    WithContext {
        child,
        phantom: PhantomData,
    }
}

// TODO: This `Debug` impl is pretty stupid
#[derive(Debug)]
pub struct WithContext<State, Action, Context: Resource, Child, ChildView> {
    child: Child,
    phantom: PhantomData<fn(State, Context) -> (Action, ChildView)>,
}

// TODO: This type shouldn't be public
#[derive(Debug)]
pub struct WithContextState<ChildState, ChildView> {
    prev: ChildView,
    child_state: ChildState,
    environment_slot: u32,
    listener_index: Option<usize>,
}

const WITH_CONTEXT_CHILD: ViewId = ViewId::new(0);

impl<State, Action, Context, Child, ChildView> ViewMarker
    for WithContext<State, Action, Context, Child, ChildView>
where
    Context: Resource,
{
}
impl<State, Action, Context, Ctx: ViewPathTracker, Child, ChildView> View<State, Action, Ctx>
    for WithContext<State, Action, Context, Child, ChildView>
where
    Child: Fn(&mut Context, &mut State) -> ChildView,
    ChildView: View<State, Action, Ctx>,
    Context: Resource,
    Self: 'static,
{
    type Element = ChildView::Element;

    type ViewState = WithContextState<ChildView::ViewState, ChildView>;

    fn build(&self, ctx: &mut Ctx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();
        ctx.with_id(WITH_CONTEXT_CHILD, |ctx| {
            let env = ctx.environment();
            let pos = env.get_slot_for_type::<Context>();
            let Some(pos) = pos else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                    core::any::type_name::<Context>()
                );
            };
            let slot_idx = usize::try_from(pos).unwrap();
            let slot = &mut env.slots[slot_idx];
            // TODO: Should this be &mut or just a shared ref?
            // If this gets modified, we won't rerun any other WithContexts for this value
            // But some types are "pure", i.e. they manage their own dependencies?
            let Some(value) = slot.item.as_mut() else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                    core::any::type_name::<Context>()
                );
            };
            let context = value
                .value
                .downcast_mut::<Context>()
                .expect("Environment's slots should have the correct types.");

            let mut first_empty = None;
            let mut needs_storing = true;
            // We store the path to this reader as a listener.
            // This is required so that we can be alerted of any changes, so that any parent
            // memoizing (or similar) views would correctly handle our value changing.
            // N.B. This is strictly only needed if:
            // 1) There actually is such a parent view
            // 2) The path for rebuilding only needs to be the path to the closest such parent
            //
            // The future changes required to enable that are already partially accounted for here (i.e. checking
            // if the current listening path is already included).
            //
            // Note also that there is currently no way to trigger these views!
            for (idx, item) in value.change_listeners.iter().enumerate() {
                if let Some(item) = item {
                    if **item == *path {
                        needs_storing = false;
                        break;
                    }
                } else {
                    first_empty.get_or_insert(idx);
                }
            }

            let listener_index = if needs_storing {
                if let Some(first_empty) = first_empty {
                    value.change_listeners[first_empty] = Some(path);
                    Some(first_empty)
                } else {
                    let idx = value.change_listeners.len();
                    value.change_listeners.push(Some(path));
                    Some(idx)
                }
            } else {
                None
            };

            let child_view = (self.child)(context, app_state);
            let (child_element, child_state) = child_view.build(ctx, app_state);

            let state = WithContextState {
                prev: child_view,
                child_state,
                environment_slot: pos,
                listener_index,
            };
            (child_element, state)
        })
    }

    fn rebuild(
        &self,
        _: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        ctx.with_id(WITH_CONTEXT_CHILD, |ctx| {
            // Use our value in the child rebuild.
            let env = ctx.environment();
            let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
            let Some(value) = slot.item.as_mut() else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                    core::any::type_name::<Context>()
                );
            };
            let context = value
                .value
                .downcast_mut::<Context>()
                .expect("Environment's slots should have the correct types.");
            let child_view = (self.child)(context, app_state);

            child_view.rebuild(
                &view_state.prev,
                &mut view_state.child_state,
                ctx,
                element,
                app_state,
            );
            view_state.prev = child_view;
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if let Some(_listener_idx) = view_state.listener_index {
            // TODO: Potentially garbage collect the listener.
            // Note that it wouldn't be correct to just remove it here, because
            // there could be multiple listeners under the same "memoization-style" view.
        }
        ctx.with_id(WITH_CONTEXT_CHILD, |ctx| {
            // TODO: We will probably want some access to the context in teardown at some point.
            view_state
                .prev
                .teardown(&mut view_state.child_state, ctx, element, app_state);
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        let Some((first, rest)) = id_path.split_first() else {
            match message.downcast::<Rebuild>() {
                Ok(_) => return MessageResult::RequestRebuild,
                Err(message) => {
                    tracing::warn!("Expected `Rebuild` in WithContext::Message, got {message:?}");
                    return MessageResult::Stale(message);
                }
            }
        };
        debug_assert_eq!(
            *first, WITH_CONTEXT_CHILD,
            "Message should have been routed properly."
        );

        // TODO: Any need for a message directly to this view?
        // TODO: When the context/environment is available in messages, add the context value here.
        view_state
            .prev
            .message(&mut view_state.child_state, rest, message, app_state)
    }
}
