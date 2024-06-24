// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, NoElement, View, ViewPathTracker};

pub fn run_once<F, Context>(once: F) -> RunOnce<F>
where
    F: Fn() + 'static,
{
    const {
        assert!(
            std::mem::size_of::<F>() == 0,
            "Using a capturing closure in `run_once` may not represent the behaviour you want.\n\
            To ignore this warning, use `run_once_raw`."
        )
    };
    RunOnce { once }
}

pub fn run_once_raw<F, Context>(once: F) -> RunOnce<F>
where
    F: Fn() + 'static,
{
    RunOnce { once }
}

pub struct RunOnce<F> {
    once: F,
}

impl<F, State, Action, Context> View<State, Action, Context> for RunOnce<F>
where
    Context: ViewPathTracker,
    F: Fn() + 'static,
{
    type Element = NoElement;

    type ViewState = ();

    fn build(&self, _: &mut Context) -> (Self::Element, Self::ViewState) {
        (self.once)();
        (NoElement, ())
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        (): &mut Self::ViewState,
        _: &mut Context,
        element: crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        // Nothing to do
        element
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        _: &mut Context,
        _: crate::Mut<'_, Self::Element>,
    ) {
        // Nothing to do
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[crate::ViewId],
        message: DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        // Nothing to do
        panic!("Message should not have been sent to a `RunOnce` View: {message:?}");
    }
}
