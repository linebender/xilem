// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The primary view trait and associated trivial implementations.

use crate::message::MessageResult;
use crate::{Arg, MessageContext, Mut, ViewArgument, ViewElement, ViewPathTracker};

/// A type which can be a [`View`]. Imposes no requirements on the underlying type.
/// Should be implemented alongside every `View` implementation:
/// ```ignore
/// impl<...> ViewMarker for Button<...> {}
/// impl<...> View<...> for Button<...> {...}
/// ```
///
/// # Details
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
pub trait View<State: ViewArgument, Action, Context: ViewPathTracker>:
    ViewMarker + 'static
{
    /// The element type which this view operates on.
    type Element: ViewElement;
    /// State that is used over the lifetime of the retained representation of the view.
    ///
    /// This often means routing information for messages to child views or view sequences,
    /// to avoid sending outdated views.
    /// This is also used in [`memoize`](crate::memoize) to store the previously constructed view.
    ///
    /// The type used for this associated type cannot be treated as public API; this is
    /// internal state to the `View` implementation.
    /// That is, `View` implementations are permitted to change the type they use for this
    ///  during even a patch release of their crate.
    type ViewState;

    /// Create the corresponding Element value.
    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState);

    /// Update `element` based on the difference between `self` and `prev`.
    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
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
        element: Mut<'_, Self::Element>,
    );

    /// Route `message` to `id_path`, if that is still a valid path.
    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action>;

    /// A view that "extracts" state from a [`View<ParentState,_,_>`] to [`View<ChildState,_,_>`].
    /// This allows modularization of views based on their state.
    ///
    /// See [`map_state`](`crate::map_state`)
    fn map_state<ParentState, F>(
        self,
        f: F,
    ) -> crate::MapState<Self, F, ParentState, State, Action, Context>
    where
        ParentState: ViewArgument,
        Action: 'static,
        Context: 'static,
        Self: Sized,
        F: for<'a> Fn(Arg<'a, ParentState>, &'a ()) -> Arg<'a, State> + 'static,
    {
        crate::map_state(self, f)
    }

    /// A view which maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`], whilst allowing the kind of [`MessageResult`] to be changed.
    ///
    /// See [`map_message`](`crate::map_message`)
    fn map_message<ParentAction, F>(
        self,
        f: F,
    ) -> crate::MapMessage<Self, State, ParentAction, Action, Context, F>
    where
        ParentAction: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(Arg<'_, State>, MessageResult<Action>) -> MessageResult<ParentAction> + 'static,
    {
        crate::map_message(self, f)
    }

    /// A view that maps a child [`View<State,ChildAction,_>`] to [`View<State,ParentAction,_>`] while providing mutable access to `State` in the map function.
    ///
    /// See [`map_action`](`crate::map_action`)
    fn map_action<ParentAction, F>(
        self,
        f: F,
    ) -> crate::MapMessage<
        Self,
        State,
        ParentAction,
        Action,
        Context,
        impl Fn(Arg<'_, State>, MessageResult<Action>) -> MessageResult<ParentAction> + 'static,
    >
    where
        ParentAction: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(Arg<'_, State>, Action) -> ParentAction + 'static,
    {
        crate::map_action(self, f)
    }

    // fn debug_name?
}
