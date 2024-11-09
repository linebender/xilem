// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Test for the behaviour of [`OrphanView<V, State, Action>`] where `V` is a view that suffers from the orphan rules.
//! This is more of a showcase how to use the `OrphanView` than a real test, as its implementation is trivial.
//!
//! This is an integration test so that it can use the infrastructure in [`common`].

#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use xilem_core::{DynMessage, MessageResult, Mut, OrphanView, View, ViewId, ViewPathTracker};

mod common;
use common::*;

/// Simple string view that increments its "generation", when it has changed.
/// This is more for documentation purposes then an actual test
impl<State, Action> OrphanView<&'static str, State, Action> for TestCtx {
    type OrphanElement = TestElement;

    type OrphanViewState = u32;

    fn orphan_build(
        _view: &&'static str,
        ctx: &mut Self,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        let id = 0;
        (
            TestElement {
                operations: vec![Operation::Build(id)],
                view_path: ctx.view_path().to_vec(),
                children: None,
            },
            id,
        )
    }

    fn orphan_rebuild(
        new: &&'static str,
        prev: &&'static str,
        generation: &mut Self::OrphanViewState,
        ctx: &mut Self,
        element: Mut<Self::OrphanElement>,
    ) {
        assert_eq!(&*element.view_path, ctx.view_path());

        let old_generation = *generation;

        if new != prev {
            *generation += 1;
        }

        element.operations.push(Operation::Rebuild {
            from: old_generation,
            to: *generation,
        });
    }

    fn orphan_teardown(
        _view: &&'static str,
        generation: &mut Self::OrphanViewState,
        _ctx: &mut Self,
        element: Mut<Self::OrphanElement>,
    ) {
        element.operations.push(Operation::Teardown(*generation));
    }

    fn orphan_message(
        _view: &&'static str,
        _view_state: &mut Self::OrphanViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

#[test]
fn str_as_orphan_view() {
    let view1 = "This string is now also a view";
    let mut ctx = TestCtx::default();
    let (mut element, mut generation) = View::<(), (), TestCtx>::build(&view1, &mut ctx);

    let view2 = "This string is now an updated view";
    assert_eq!(element.operations[0], Operation::Build(0));
    View::<(), (), TestCtx>::rebuild(&view1, &view2, &mut generation, &mut ctx, &mut element);
    assert_eq!(element.operations[1], Operation::Rebuild { from: 0, to: 1 });
    View::<(), (), TestCtx>::teardown(&view1, &mut generation, &mut ctx, &mut element);
    assert_eq!(element.operations[2], Operation::Teardown(1));
}
