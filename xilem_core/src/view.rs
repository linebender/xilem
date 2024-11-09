// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The primary view trait and associated trivial implementations.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::ops::Deref;

use crate::message::MessageResult;
use crate::{DynMessage, Mut, ViewElement};

/// A type which can be a [`View`]. Imposes no requirements on the underlying type.
/// Should be implemented alongside every `View` implementation:
/// ```ignore
/// impl<...> ViewMarker for Button<...> {}
/// impl<...> View<...> for Button<...> {...}
/// ```
///
/// ## Details
///
/// Because `View` is generic, Rust [allows you](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules) to implement this trait for certain non-local types.
/// These non-local types can include `Vec<_>` and `Option<_>`.
/// If this trait were not present, those implementations of `View` would conflict with those types' implementations of `ViewSequence`.
/// This is because every `View` type also implementations `ViewSequence`.
/// Since `ViewMarker` is not generic, these non-local implementations are not permitted for this trait, which means that the conflicting implementation cannot happen.
pub trait ViewMarker {}

/// A lightweight, short-lived representation of the state of a retained
/// structure, usually a user interface node.
///
/// This is the central reactivity primitive in Xilem.
/// An app will generate a tree of these objects (the view tree) to represent
/// the state it wants to show in its element tree.
/// The framework will then run methods on these views to create the associated
/// element tree, or to perform incremental updates to the element tree.
/// Once this process is complete, the element tree will reflect the view tree.
/// The view tree is also used to dispatch messages, such as those sent when a
/// user presses a button.
///
/// The view tree is transitory and is retained only long enough to dispatch
/// messages and then serve as a reference for diffing for the next view tree.
///
/// The `View` trait is parameterized by `State`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in message
/// propagation.
/// During message handling, mutable access to the app state is given to view nodes,
/// which will in turn generally expose it to callbacks.
///
/// Due to restrictions of the [orphan rules](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules),
/// `ViewMarker` needs to be implemented for every type that implements `View`, see [`ViewMarker`] for more details.
/// For example:
/// ```ignore
/// impl<...> ViewMarker for Button<...> {}
/// impl<...> View<...> for Button<...> {...}
/// ```
///
/// ## Alloc
///
/// In order to support the default open-ended [`DynMessage`] type as `Message`, this trait requires an
/// allocator to be available.
pub trait View<State, Action, Context: ViewPathTracker, Message = DynMessage>:
    ViewMarker + 'static
{
    /// The element type which this view operates on.
    type Element: ViewElement;
    /// State that is used over the lifetime of the retained representation of the view.
    ///
    /// This often means routing information for messages to child views or view sequences,
    /// to avoid sending outdated views.
    /// This is also used in [`memoize`](crate::memoize) to store the previously constructed view.
    type ViewState;

    /// Create the corresponding Element value.
    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState);

    /// Update `element` based on the difference between `self` and `prev`.
    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    );

    /// Handle `element` being removed from the tree.
    ///
    /// The main use-cases of this method are to:
    /// - Cancel any async tasks
    /// - Clean up any book-keeping set-up in `build` and `rebuild`
    // TODO: Should this take ownership of the `ViewState`
    // We have chosen not to because it makes swapping versions more awkward
    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    );

    /// Route `message` to `id_path`, if that is still a valid path.
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message>;

    // fn debug_name?
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// An identifier used to differentiation between the direct children of a [`View`].
///
/// These are [`u64`] backed identifiers, which will be added to the "view path" in
/// [`View::build`] and [`View::rebuild`] (and their [`ViewSequence`](crate::ViewSequence) counterparts),
/// and removed from the start of the path if necessary in [`View::message`].
/// The value of `ViewId`s are only meaningful for the `View` or `ViewSequence` added them
/// to the path, and can be used to store indices and/or generations.
// TODO: maybe also provide debugging information to give e.g. a useful stack trace?
// TODO: Rethink name, as 'Id' suggests global uniqueness
pub struct ViewId(u64);

impl ViewId {
    /// Create a new `ViewId` with the given value.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Access the raw value of this id.
    #[must_use]
    pub const fn routing_id(self) -> u64 {
        self.0
    }
}

/// A tracker for view paths, used in [`View::build`] and [`View::rebuild`].
/// These paths are used for routing messages in [`View::message`].
///
/// Each `View` is expected to be implemented for one logical context type,
/// and this context may be used to store auxiliary data.
/// For example, this context could be used to store a mapping from the
/// id of widget to view path, to enable event routing.
pub trait ViewPathTracker {
    /// Add `id` to the end of current view path
    fn push_id(&mut self, id: ViewId);
    /// Remove the most recently `push`ed id from the current view path
    fn pop_id(&mut self);

    /// The path to the current view in the view tree
    fn view_path(&mut self) -> &[ViewId];

    /// Run `f` in a context with `id` pushed to the current view path
    fn with_id<R>(&mut self, id: ViewId, f: impl FnOnce(&mut Self) -> R) -> R {
        self.push_id(id);
        let res = f(self);
        self.pop_id();
        res
    }
}

impl<V: ?Sized> ViewMarker for Box<V> {}
impl<State, Action, Context, Message, V> View<State, Action, Context, Message> for Box<V>
where
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message> + ?Sized,
{
    type Element = V::Element;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.deref().build(ctx)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        self.deref().rebuild(prev, view_state, ctx, element);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        self.deref().teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        self.deref()
            .message(view_state, id_path, message, app_state)
    }
}

#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
pub struct ArcState<ViewState> {
    view_state: ViewState,
    dirty: bool,
}

impl<V: ?Sized> ViewMarker for Arc<V> {}
/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context, Message, V> View<State, Action, Context, Message> for Arc<V>
where
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message> + ?Sized,
{
    type Element = V::Element;
    type ViewState = ArcState<V::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let (element, view_state) = self.deref().build(ctx);
        (
            element,
            ArcState {
                view_state,
                dirty: false,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        // If this is the same value, or no rebuild was forced, there's no need to rebuild
        if core::mem::take(&mut view_state.dirty) || !Arc::ptr_eq(self, prev) {
            self.deref()
                .rebuild(prev, &mut view_state.view_state, ctx, element);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        self.deref()
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let message_result =
            self.deref()
                .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}

impl<V: ?Sized> ViewMarker for Rc<V> {}
/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context, Message, V> View<State, Action, Context, Message> for Rc<V>
where
    Context: ViewPathTracker,
    V: View<State, Action, Context, Message> + ?Sized,
{
    type Element = V::Element;
    type ViewState = ArcState<V::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let (element, view_state) = self.deref().build(ctx);
        (
            element,
            ArcState {
                view_state,
                dirty: false,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        // If this is the same value, or no rebuild was forced, there's no need to rebuild
        if core::mem::take(&mut view_state.dirty) || !Rc::ptr_eq(self, prev) {
            self.deref()
                .rebuild(prev, &mut view_state.view_state, ctx, element);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<Self::Element>,
    ) {
        self.deref()
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Message,
        app_state: &mut State,
    ) -> MessageResult<Action, Message> {
        let message_result =
            self.deref()
                .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}
