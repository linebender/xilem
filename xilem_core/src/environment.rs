// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Values accessible throughout the Xilem view tree.

use anymore::AnyDebug;
use hashbrown::{HashMap, hash_map::Entry};

use crate::{
    Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewId, ViewMarker, ViewPathTracker,
};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{any::TypeId, marker::PhantomData};

#[derive(Debug)]
/// A message sent to Views to instruct them to rebuild themselves.
///
/// This will be sent when a value in the environment value is modified.
pub struct Rebuild;

// --- MARK: Environment

#[derive(Debug)]
#[expect(missing_docs, reason = "Public on an interim basis")]
pub struct EnvironmentItem {
    #[expect(missing_docs, reason = "Public on an interim basis")]
    pub value: Box<dyn AnyDebug>,
    // TODO: Can we/do we want to make these share an allocation?
    // TODO: How do we GC this?
    // TODO: The change listeners currently aren't ever notified.
    change_listeners: Vec<Option<Arc<[ViewId]>>>,
}

#[derive(Debug)]
#[expect(missing_docs, reason = "Public on an interim basis")]
pub struct Slot {
    #[expect(missing_docs, reason = "Public on an interim basis")]
    pub item: Option<EnvironmentItem>,
    ref_count: u32,
    // generation: u32,
}

/// A store of values which are accessible throughout the view tree.
///
/// Values can be made available to any child views using [`provides`],
/// then read using [`with_context`].
/// This type is the internal implementation detail of these views, and
/// currently can only meaningfully be used directly by those types.
///
/// This type is owned by the Xilem driver, and so shouldn't be created by end users.
/// This type must be made available by view contexts through the
/// [`environment`](ViewPathTracker::environment)  method of `ViewPathTracker`, which
/// they all implement.
#[derive(Debug)]
pub struct Environment {
    #[expect(missing_docs, reason = "Public on an interim basis")]
    pub slots: Vec<Slot>,
    // We use u32 here so that we could move to a generation
    free_slots: Vec<u32>,
    types: HashMap<TypeId, u32>,
    // TODO: Think about how to handle this.
    // queued_rebuilds: Vec<Arc<[ViewId]>>,
}

impl Environment {
    /// Create a new `Environment`.
    ///
    /// End-users of Xilem do not need to use this function.
    /// For driver implementers, there should only ever be one environment throughout
    /// the lifecycle of your driver.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            types: HashMap::new(),
            // queued_rebuilds: Vec::new(),
        }
    }

    // TODO: Possibly reconsider the name here.
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

    #[expect(missing_docs, reason = "Public on an interim basis")]
    pub fn get_slot_for_type<Context>(&mut self) -> Option<u32>
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
///
/// Types which implement this trait can be made available to child views
/// using [`provides`], and then read using [`with_context`].
///
/// The preferred variable name for types bounded by this type is currently
/// `Context`, based on the name used in React for their similar feature.
// TODO: Make sure that these names make sense.
pub trait Resource: AnyDebug {}

// --- MARK: Provides

/// View which makes a [`Resource`] value of a specific type available to all of its descendants.
///
/// The provided `initial_context` will be called as soon as the returned view is
/// built, to get the initial value of the resource.
/// This context value can be read using the [`with_context`] view with the same
/// `Context` type parameter within child.
///
/// Note that it is not currently fully supported to mutate `Resource` values.
/// In particular, if these values are read inside memoising views (such as
/// [`memoize`](crate::memoize)), the values won't be correctly updated.
/// We intend to address this limitation in the future.
///
/// This is analogous to `Context.Provider` in React.
pub fn provides<State, Action, Context, InitialContext, ChildView, Ctx>(
    initial_context: InitialContext,
    child: ChildView,
) -> Provides<State, Action, Context, InitialContext, ChildView>
where
    State: ViewArgument,
    InitialContext: Fn(Arg<'_, State>) -> Context,
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
#[must_use = "View values do nothing unless provided to Xilem."]
/// The View type for [`provides`]. See its documentation for details.
pub struct Provides<State, Action, Context: Resource, InitialContext, ChildView> {
    initial_context: InitialContext,
    child: ChildView,
    phantom: PhantomData<fn(State, Context) -> Action>,
}

#[derive(Debug)]
#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
pub struct ProvidesState<ChildState> {
    child_state: ChildState,
    this_state: Option<EnvironmentItem>,
    environment_slot: u32,
}

impl<State, Action, Context, InitialContext, ChildView> ViewMarker
    for Provides<State, Action, Context, InitialContext, ChildView>
where
    State: ViewArgument,
    Context: Resource,
{
}

impl<State, Action, Context, InitialContext, Ctx: ViewPathTracker, ChildView>
    View<State, Action, Ctx> for Provides<State, Action, Context, InitialContext, ChildView>
where
    State: ViewArgument,
    InitialContext: Fn(Arg<'_, State>) -> Context,
    ChildView: View<State, Action, Ctx>,
    Context: Resource,
    Self: 'static,
{
    type Element = ChildView::Element;

    type ViewState = ProvidesState<ChildView::ViewState>;

    fn build(
        &self,
        ctx: &mut Ctx,
        mut app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        // Prepare the initial state value
        let value = (self.initial_context)(State::reborrow_mut(&mut app_state));
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
        let (child_element, child_state) =
            self.child.build(ctx, State::reborrow_mut(&mut app_state));

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
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
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
        element: Mut<'_, Self::Element>,
    ) {
        // Make our value available in the child teardown.
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        self.child
            .teardown(&mut view_state.child_state, ctx, element);

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
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        // Use our value in the child message.
        let slot =
            &mut message.environment.slots[usize::try_from(view_state.environment_slot).unwrap()];
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should be providing something."
        );
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        // TODO: Any need for a message directly to this view?
        // TODO: When the context/environment is available in messages, add the context value here.
        let ret = self
            .child
            .message(&mut view_state.child_state, message, element, app_state);

        let slot =
            &mut message.environment.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should get its value back."
        );
        ret
    }
}

// --- MARK: WithContext

/// View which gives access to a [`Resource`] value from its environment.
///
/// The provided `child` function will be called to create the view which represents the
/// target state of this view's location in the tree.
/// When it is called, `child` will be provided with the current app state, and the
/// current value of type `Context` from the environment.
/// This `Context` value can be inserted into the environment using [`provides`], which
/// must be a parent view of this view.
/// This view will read the resource value from the closest ancestor `provides`.
/// If there is no such ancestor, this view will panic when it is built (or rebuilt).
///
/// Note that it is not currently fully supported to mutate `Resource` values.
/// In particular, if these values are read inside memoising views (such as
/// [`memoize`](crate::memoize)), the values won't be correctly updated.
/// We intend to address this limitation in the future.
///
/// This is analogous to `Context.Consumer` in React.
pub fn with_context<State, Action, Context, Child, ChildView, Ctx>(
    child: Child,
) -> WithContext<State, Action, Context, Child, ChildView>
where
    State: ViewArgument,
    Child: Fn(&mut Context, Arg<'_, State>) -> ChildView,
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
#[must_use = "View values do nothing unless provided to Xilem."]
/// The View type for [`with_context`]. See its documentation for details.
pub struct WithContext<State, Action, Context: Resource, Child, ChildView> {
    child: Child,
    phantom: PhantomData<fn(State, Context) -> (Action, ChildView)>,
}

#[derive(Debug)]
#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
pub struct WithContextState<ChildState, ChildView> {
    prev: ChildView,
    child_state: ChildState,
    environment_slot: u32,
    listener_index: Option<usize>,
}

// Use a distinctive number here, to be able to catch bugs.
/// This is a randomly generated 32 bit number - 3326962411 in decimal.
const WITH_CONTEXT_CHILD: ViewId = ViewId::new(0xc64d6aeb);

impl<State, Action, Context, Child, ChildView> ViewMarker
    for WithContext<State, Action, Context, Child, ChildView>
where
    State: ViewArgument,
    Context: Resource,
{
}
impl<State, Action, Context, Ctx: ViewPathTracker, Child, ChildView> View<State, Action, Ctx>
    for WithContext<State, Action, Context, Child, ChildView>
where
    State: ViewArgument,
    Child: Fn(&mut Context, Arg<'_, State>) -> ChildView,
    ChildView: View<State, Action, Ctx>,
    Context: Resource,
    Self: 'static,
{
    type Element = ChildView::Element;

    type ViewState = WithContextState<ChildView::ViewState, ChildView>;

    fn build(
        &self,
        ctx: &mut Ctx,
        mut app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();
        ctx.with_id(WITH_CONTEXT_CHILD, |ctx| {
            let env = ctx.environment();
            let pos = env.get_slot_for_type::<Context>();
            let Some(pos) = pos else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been provided. Did you forget to wrap this view with `xilem_core::environment::provides`?",
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

            let child_view = (self.child)(context, State::reborrow_mut(&mut app_state));
            let (child_element, child_state) = child_view.build(ctx, State::reborrow_mut(&mut app_state));

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
        element: Mut<'_, Self::Element>,
        mut app_state: Arg<'_, State>,
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
            let child_view = (self.child)(context, State::reborrow_mut(&mut app_state));

            child_view.rebuild(
                &view_state.prev,
                &mut view_state.child_state,
                ctx,
                element,
                State::reborrow_mut(&mut app_state),
            );
            view_state.prev = child_view;
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Ctx,
        element: Mut<'_, Self::Element>,
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
                .teardown(&mut view_state.child_state, ctx, element);
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let Some(first) = message.take_first() else {
            match message.take_message::<Rebuild>() {
                Some(_) => return MessageResult::RequestRebuild,
                None => {
                    tracing::warn!("Expected `Rebuild` in WithContext::Message, got {message:?}");
                    return MessageResult::Stale;
                }
            }
        };
        debug_assert_eq!(
            first, WITH_CONTEXT_CHILD,
            "Message should have been routed properly."
        );

        view_state
            .prev
            .message(&mut view_state.child_state, message, element, app_state)
    }
}

// --- MARK: OnActionWithContext

// TODO: This `Debug` impl is pretty stupid
#[derive(Debug)]
#[must_use = "View values do nothing unless provided to Xilem."]
/// The View type for [`on_action_with_context`]. See its documentation for details.
pub struct OnActionWithContext<State, Action, Context, OnAction, Res, ChildView, ChildAction> {
    child: ChildView,
    on_action: OnAction,
    phantom: PhantomData<fn(State, ChildAction, Context, Res) -> Action>,
}

/// Operate on an environment value when a child view returns an action.
///
/// This is an interim solution whilst we design APIs for environment
/// manipulation.
///
/// The first argument `on_action` is the function which will be ran in this case.
/// The arguments are the app's state, the specified resource and the action returned
/// by the child.
pub fn on_action_with_context<State, Action, Context, OnAction, Res, ChildView, ChildAction>(
    on_action: OnAction,
    child: ChildView,
) -> OnActionWithContext<State, Action, Context, OnAction, Res, ChildView, ChildAction>
where
    State: ViewArgument,
    Context: ViewPathTracker,
    // Experiment:
    OnActionWithContext<State, Action, Context, OnAction, Res, ChildView, ChildAction>:
        View<State, Action, Context>,
    OnAction: Fn(Arg<'_, State>, &mut Res, ChildAction) -> Action,
{
    OnActionWithContext {
        child,
        on_action,
        phantom: PhantomData,
    }
}

#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
#[derive(Debug)]
pub struct OnActionWithContextState<ChildState> {
    child_state: ChildState,
    environment_slot: u32,
}

impl<State, Action, Context, OnAction, Res, ChildView, ChildAction> ViewMarker
    for OnActionWithContext<State, Action, Context, OnAction, Res, ChildView, ChildAction>
{
}
impl<State, Action, Context, OnAction, Res, ChildView, ChildAction> View<State, Action, Context>
    for OnActionWithContext<State, Action, Context, OnAction, Res, ChildView, ChildAction>
where
    State: ViewArgument,
    Context: ViewPathTracker,
    Res: Resource,
    Self: 'static,
    ChildView: View<State, ChildAction, Context>,
    OnAction: Fn(Arg<'_, State>, &mut Res, ChildAction) -> Action,
{
    type Element = ChildView::Element;

    type ViewState = OnActionWithContextState<ChildView::ViewState>;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (element, child_state) = self.child.build(ctx, app_state);
        let env = ctx.environment();
        let pos = env.get_slot_for_type::<Res>();
        let Some(pos) = pos else {
            panic!(
                // TODO: Track caller for this view?
                "Xilem: Tried to get context for {}, but it hasn't been provided. Did you forget to wrap this view with `xilem_core::environment::provides`?",
                core::any::type_name::<Context>()
            );
        };
        (
            element,
            OnActionWithContextState {
                child_state,
                environment_slot: pos,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        self.child.rebuild(
            &prev.child,
            &mut view_state.child_state,
            ctx,
            element,
            app_state,
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.child
            .teardown(&mut view_state.child_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        mut app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let prev_res = self.child.message(
            &mut view_state.child_state,
            message,
            element,
            State::reborrow_mut(&mut app_state),
        );
        // Use our value in the child rebuild.

        let env = &mut message.environment;
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        let Some(value) = slot.item.as_mut() else {
            panic!(
                // TODO: Track caller for this view?
                "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                core::any::type_name::<Res>()
            );
        };
        let resource = value
            .value
            .downcast_mut::<Res>()
            .expect("Environment's slots should have the correct types.");

        prev_res.map(|child_action| {
            (self.on_action)(State::reborrow_mut(&mut app_state), resource, child_action)
        })
    }
}
