// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// Hide these docs from "general audiences" online.
// but keep them available for developers of Xilem Core to browse.
#![cfg_attr(docsrs, doc(hidden))]

//! Fake implementations of Xilem traits for use within documentation examples and tests.
//!
//! Users of Xilem Core should not use these traits.
//!
//! The items defined in this trait will often be imported in doc comments in renamed form.
//! This is mostly intended for writing documentation internally to Xilem Core.
//!
//! This module is not required to follow semver. It is public only for documentation purposes.
//!
//! # Examples
//!
//! ```
//! /// A view to do something fundamental
//! ///
//! /// # Examples
//! /// ```
//! /// # use xilem_core::docs::{DocsView as WidgetView, State};
//! /// use xilem_core::interesting_primitive;
//! /// fn user_component() -> WidgetView<State> {
//! ///     interesting_primitive()
//! /// }
//! ///
//! /// ```
//! fn interesting_primitive() -> InterestingPrimitive {
//!    // ...
//! #  InterestingPrimitive
//! }
//! # struct InterestingPrimitive;
//! ```

use crate::{run_once, View, ViewPathTracker};

/// A type used for documentation
pub enum Fake {}

impl ViewPathTracker for Fake {
    fn push_id(&mut self, _: crate::ViewId) {
        match *self {}
    }
    fn pop_id(&mut self) {
        match *self {}
    }

    fn view_path(&mut self) -> &[crate::ViewId] {
        match *self {}
    }
}

/// A version of [`View`] used for documentation.
///
/// This will often be imported by a different name in a hidden use item.
///
/// In most cases, that name will be `WidgetView`, as Xilem Core's documentation is
/// primarily targeted at users of [Xilem](https://crates.io/crates/xilem/).
pub trait DocsView<State, Action = ()>: View<State, Action, Fake> {}
impl<V, State, Action> DocsView<State, Action> for V where V: View<State, Action, Fake> {}

/// A state type usable in a component
pub struct State;

/// A minimal component.
pub fn some_component<Action>(_: &mut State) -> impl DocsView<State, Action> {
    // The view which does nothing already exists in `run_once`.
    run_once(|| {})
}
