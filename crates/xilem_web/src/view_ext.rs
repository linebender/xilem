// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use crate::{view::View, Adapt, AdaptState, AdaptThunk};

/// A trait that makes it possible to use core views such as [`Adapt`] in the continuation/builder style.
pub trait ViewExt<T, A>: View<T, A> + Sized {
    fn adapt<ParentT, ParentA, F>(self, f: F) -> Adapt<ParentT, ParentA, T, A, Self, F>
    where
        F: Fn(&mut ParentT, AdaptThunk<T, A, Self>) -> xilem_core::MessageResult<ParentA>,
    {
        Adapt::new(f, self)
    }

    fn adapt_state<ParentT, F>(self, f: F) -> AdaptState<ParentT, T, Self, F>
    where
        F: Fn(&mut ParentT) -> &mut T + Send,
    {
        AdaptState::new(f, self)
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {}
