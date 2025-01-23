// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests of the primary [`ViewSequence`] implementations
//!
//! [`ViewSequence`]: xilem_core::ViewSequence

#![expect(
    clippy::shadow_unrelated,
    reason = "Deferred: Noisy. Fix is to use scopes"
)]

mod common;
use common::*;
use xilem_core::{MessageResult, View};

fn record_ops(id: u32) -> OperationView<0> {
    OperationView(id)
}

/// The implicit sequence of a single View should forward all operations
#[test]
fn one_element_sequence_passthrough() {
    let view = sequence(1, record_ops(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(1)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    assert_eq!(child.operations, &[Operation::Build(0)]);
    assert_eq!(
        child.view_path,
        &[],
        "The single `View` ViewSequence shouldn't add to the view path"
    );

    let view2 = sequence(3, record_ops(2));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(
        element.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 3 }]
    );

    assert_eq!(seq_children.active.len(), 1);
    assert!(seq_children.deleted.is_empty());
    let child = seq_children.active.first().unwrap();
    assert_eq!(
        child.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 2 }]
    );

    let result = view2.message(&mut state, &[], Box::new(()), &mut ());
    // The message should have been routed to the only child
    assert_action(result, 2);

    view2.teardown(&mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[
            Operation::Build(1),
            Operation::Rebuild { from: 1, to: 3 },
            Operation::Teardown(3)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    // It has been removed from the parent sequence when tearing down
    assert_eq!(seq_children.active.len(), 0);
    assert_eq!(seq_children.deleted.len(), 1);
    let (child_idx, child) = seq_children.deleted.first().unwrap();
    assert_eq!(*child_idx, 0);
    assert_eq!(
        child.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 2 },
            Operation::Teardown(2)
        ]
    );
}

#[test]
fn option_none_none() {
    let view = sequence(0, None::<OperationView<0>>);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert!(seq_children.active.is_empty());

    let view2 = sequence(1, None);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 1 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert!(seq_children.active.is_empty());

    view2.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 1 },
            Operation::Teardown(1)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert!(seq_children.active.is_empty());
}

#[test]
fn option_some_some() {
    let view = sequence(1, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(1)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    assert_eq!(child.operations, &[Operation::Build(0)]);
    // Option is allowed (and expected) to add to the view path
    assert_eq!(child.view_path.len(), 1);

    let view2 = sequence(3, Some(record_ops(2)));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 3 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    assert_eq!(
        child.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 2 }]
    );

    view2.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(1),
            Operation::Rebuild { from: 1, to: 3 },
            Operation::Teardown(3)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.deleted.len(), 1);
    assert!(seq_children.active.is_empty());
    let (child_idx, child) = seq_children.deleted.first().unwrap();
    assert_eq!(*child_idx, 0);
    assert_eq!(
        child.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 2 },
            Operation::Teardown(2)
        ]
    );
}

#[test]
fn option_none_some() {
    let view = sequence(0, None);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert!(seq_children.active.is_empty());

    let view2 = sequence(2, Some(record_ops(1)));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 2 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    assert_eq!(child.operations, &[Operation::Build(1)]);

    view2.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 2 },
            Operation::Teardown(2)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.deleted.len(), 1);
    assert!(seq_children.active.is_empty());
    let (child_idx, child) = seq_children.deleted.first().unwrap();
    assert_eq!(*child_idx, 0);
    assert_eq!(
        child.operations,
        &[Operation::Build(1), Operation::Teardown(1)]
    );
}

#[test]
fn option_some_none() {
    let view = sequence(1, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(1)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    assert_eq!(child.operations, &[Operation::Build(0)]);
    // Option is allowed (and expected) to add to the view path
    assert_eq!(child.view_path.len(), 1);

    let view2 = sequence(2, None);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 2 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.deleted.len(), 1);
    assert!(seq_children.active.is_empty());
    let (child_idx, child) = seq_children.deleted.first().unwrap();
    assert_eq!(*child_idx, 0);
    assert_eq!(
        child.operations,
        &[Operation::Build(0), Operation::Teardown(0)]
    );

    view2.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(1),
            Operation::Rebuild { from: 1, to: 2 },
            Operation::Teardown(2)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.deleted.len(), 1);
    assert!(seq_children.active.is_empty());
}

#[test]
fn option_message_some() {
    let view = sequence(1, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    let path = child.view_path.to_vec();

    let result = view.message(&mut state, &path, Box::new(()), &mut ());
    assert_action(result, 0);
}

#[test]
fn option_message_some_some() {
    let view = sequence(0, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    let path = child.view_path.to_vec();

    let view2 = sequence(0, Some(record_ops(1)));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);

    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert_action(result, 1);
}

#[test]
fn option_message_some_none_stale() {
    let view = sequence(0, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    let path = child.view_path.to_vec();

    let view2 = sequence(0, None);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);

    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}

#[test]
fn option_message_some_none_some_stale() {
    let view = sequence(0, Some(record_ops(0)));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();

    let seq_children = element.children.as_ref().unwrap();
    assert_eq!(seq_children.active.len(), 1);
    let child = seq_children.active.first().unwrap();
    let path = child.view_path.to_vec();

    let view2 = sequence(0, None);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);

    let view3 = sequence(0, Some(record_ops(1)));
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element);

    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}
