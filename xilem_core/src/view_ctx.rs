// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::environment::Environment;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// An identifier used to differentiate between the direct children of a [`View`].
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

// TODO - Rename to ViewCtx

/// A tracker for view paths, used in [`View::build`] and [`View::rebuild`].
/// These paths are used for routing messages in [`View::message`].
///
/// Each `View` is expected to be implemented for one logical context type,
/// and this context may be used to store auxiliary data.
/// For example, this context could be used to store a mapping from the
/// id of widget to view path, to enable event routing.
pub trait ViewPathTracker {
    /// Access the [`Environment`] associated with this context.
    ///
    /// I hope that we can remove the "context" generic entirely, and so this is here
    /// on a temporary basis.
    fn environment(&mut self) -> &mut Environment;
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
