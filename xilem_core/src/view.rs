// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The primary view trait and associated trivial implementations.

use core::ops::Deref;

use alloc::{boxed::Box, sync::Arc};

use crate::{message::MessageResult, DynMessage, ViewElement};

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
/// ## Alloc
///
/// In order to support the open-ended [`DynMessage`] type, this trait requires an
/// allocator to be available.
/// It is possible (hopefully in a backwards compatible way) to add a generic
/// defaulted parameter for the message type in future.
pub trait View<State, Action, Context: ViewPathTracker>: 'static {
    /// The element type which this view operates on.
    type Element: ViewElement;
    /// The state needed for this view to route messages to the correct child view.
    type ViewState;

    /// Create the corresponding Element value.
    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState);

    /// Update `element` based on the difference between `self` and `prev`.
    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as ViewElement>::Mut<'_>,
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
        element: <Self::Element as ViewElement>::Mut<'_>,
    );

    /// Route `message` to `id_path`, if that is still a valid path.
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>;

    // fn debug_name?
}

#[derive(Copy, Clone, Debug)]
/// An identifier for a subtree in a view hierarchy.
// TODO: also provide debugging information to give e.g. a useful stack trace?
pub struct ViewId(u64);

impl ViewId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn routing_id(self) -> u64 {
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

impl<State, Action, Context: ViewPathTracker, V: View<State, Action, Context> + ?Sized>
    View<State, Action, Context> for Box<V>
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
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        self.deref().rebuild(prev, view_state, ctx, element);
    }
    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        self.deref().teardown(view_state, ctx, element);
    }
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.deref()
            .message(view_state, id_path, message, app_state)
    }
}

/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context: ViewPathTracker, V: View<State, Action, Context> + ?Sized>
    View<State, Action, Context> for Arc<V>
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
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        if !Arc::ptr_eq(self, prev) {
            self.deref().rebuild(prev, view_state, ctx, element);
        }
    }
    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        self.deref().teardown(view_state, ctx, element);
    }
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.deref()
            .message(view_state, id_path, message, app_state)
    }
}
