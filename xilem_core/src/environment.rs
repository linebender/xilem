// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Values accessible throughout the Xilem view tree.

use core::{any::TypeId, marker::PhantomData};

use crate::{AnyMessage, View, ViewId, ViewMarker, ViewPathTracker};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use hashbrown::{HashMap, hash_map::Entry};

#[derive(Debug)]
pub struct Rebuild;

struct EnvironmentItem {
    value: Box<dyn AnyMessage>,
    // TODO: Can we/do we want to make these share an allocation?
    change_listeners: Vec<Option<Arc<[ViewId]>>>,
}

struct Slot {
    item: Option<EnvironmentItem>,
    ref_count: u32,
    // generation: u32,
}

pub struct Environment {
    slots: Vec<Slot>,
    // We use u32 here so that we could move to a generation
    free_slots: Vec<u32>,
    types: HashMap<TypeId, u32>,
    // TODO: Think about this more carefully
    queued_rebuilds: Vec<Arc<[ViewId]>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            types: HashMap::new(),
            queued_rebuilds: Vec::new(),
        }
    }

    fn slot_for_type<Context>(&mut self) -> u32
    where
        Context: Resource,
    {
        let pos = match self.types.entry(TypeId::of::<Context>()) {
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
        };
        pos
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker trait for types usable as resources.
pub trait Resource: AnyMessage {}

// TODO: This `Debug` impl is pretty stupid
#[derive(Debug)]
pub struct Provides<State, Action, Context: Resource, InitialContext, Child> {
    initial_context: InitialContext,
    child: Child,
    phantom: PhantomData<fn(State, Context) -> Action>,
}

pub struct ProvidesState<ChildState, ChildView> {
    prev: ChildView,
    child_state: ChildState,
    this_state: Option<EnvironmentItem>,
    environment_slot: u32,
}

impl<State, Action, Context, InitialContext, Child> ViewMarker
    for Provides<State, Action, Context, InitialContext, Child>
where
    Context: Resource,
{
}

impl<State, Action, Context, InitialContext, Child, Ctx: ViewPathTracker, ChildView>
    View<State, Action, Ctx> for Provides<State, Action, Context, InitialContext, Child>
where
    InitialContext: Fn(&mut State) -> Context,
    // TODO: Thinking about it, this being a function is unnecessary because the only way to
    // access a context value within the child is something which is itself a function.
    Child: Fn(&mut State) -> ChildView,
    ChildView: View<State, Action, Ctx>,
    Context: Resource,
    Self: 'static,
{
    type Element = ChildView::Element;

    type ViewState = ProvidesState<ChildView::ViewState, ChildView>;

    fn build(&self, ctx: &mut Ctx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // Prepare the initial state value
        let value = (self.initial_context)(app_state);
        let environment_item = EnvironmentItem {
            change_listeners: Vec::new(),
            value: Box::new(value),
        };

        let env = ctx.environment();
        let pos = env.slot_for_type::<Context>();

        let child_view = (self.child)(app_state);

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
        let (child_element, child_state) = child_view.build(ctx, app_state);

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
            prev: child_view,
            child_state,
            this_state: Some(my_item),
            environment_slot: pos,
        };
        (child_element, state)
    }

    fn rebuild(
        &self,
        _: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        // Use our value in the child rebuild.
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        let child_view = (self.child)(app_state);
        child_view.rebuild(
            &view_state.prev,
            &mut view_state.child_state,
            ctx,
            element,
            app_state,
        );
        view_state.prev = child_view;

        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: crate::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        todo!()
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        // TODO: Any need for a message directly to this view?
        // TODO: When the context is available in messages, add the context value here.
        view_state
            .prev
            .message(&mut view_state.child_state, id_path, message, app_state)
    }
}
