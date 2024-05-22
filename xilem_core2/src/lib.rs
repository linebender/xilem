// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![no_std]
// TODO: Point at documentation for this pattern of README include
#![doc = concat!(
" 
<!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
[LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_core/LICENSE

<!-- intra-doc-links go here -->
<!-- TODO: If the alloc feature is disabled, this link doesn't resolve -->
[`alloc`]: alloc

<style>
.rustdoc-hidden { display: none; }
</style>

<!-- Hide the header section of the README when using rustdoc -->
<div style=\"display:none\">
",
    include_str!("../README.md"),
)]

extern crate alloc;

mod element;
pub use element::{Element, SuperElement};

mod id;
pub use id::ViewId;

mod any_view;
pub use any_view::AnyView;

mod message;
pub use message::{DynMessage, Message};

mod model;

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
/// It is also
///
/// ## Alloc
///
/// In order to support the open-ended [`DynMessage`] type, this trait requires an
/// allocator to be available.
/// It is possible (hopefully in a backwards compatible way) to add a generic
/// defaulted parameter for the message type in future.
pub trait View<State, Action, Context: ViewPathTracker>: 'static {
    /// The element type which this view operates on.
    type Element: Element;
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
        element: <Self::Element as Element>::Mut<'_>,
    );

    /// Handle `element` being removed from the tree.
    ///
    /// The main use-case of this method is to:
    /// 1) Cancel any async task
    /// 2) Clean up any book-keeping set-up in `build` and `rebuild`
    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as Element>::Mut<'_>,
    );

    /// Route `message` to `id_path`, if that is still a valid path.
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

/// The possible outcomes from a [`View::message`]
#[derive(Default)]
pub enum MessageResult<Action> {
    /// An action for a parent message handler to use
    ///
    /// This allows for sub-sections of your app to use an elm-like architecture
    Action(Action),
    /// This event had no impact on the app state, or the impact it did have
    /// does not require the element tree to be recreated.
    RequestRebuild,
    #[default]
    /// This event had no impact on the app state, or the impact it did have
    /// does not require the element tree to be recreated.
    Nop,
    /// The view this message was being routed to no longer exists.
    Stale(DynMessage),
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

/* /// Types which can route a message view to a child [`View`].
// TODO: This trait needs to exist for desktop hot reloading
pub trait ViewMessage<State, Action> {
    type ViewState;
}
*/
