// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

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

#[test]
fn zero_zero() {
    let view = sequence(0, Vec::<OperationView<0>>::new());
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert!(seq_children.active.is_empty());

    let view2 = sequence(1, vec![]);
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
fn one_zero() {
    let view = sequence(1, vec![record_ops(0)]);
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
    assert_eq!(child.view_path.len(), 1);

    let view2 = sequence(2, vec![]);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 2 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.active.is_empty());
    assert_eq!(seq_children.deleted.len(), 1);
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
    assert!(seq_children.active.is_empty());
    assert_eq!(seq_children.deleted.len(), 1);
    let (child_idx, child) = seq_children.deleted.first().unwrap();
    assert_eq!(*child_idx, 0);
    assert_eq!(
        child.operations,
        &[Operation::Build(0), Operation::Teardown(0)]
    );
}

#[test]
fn one_two() {
    let view = sequence(1, vec![record_ops(0)]);
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
    assert_eq!(child.view_path.len(), 1);

    let view2 = sequence(4, vec![record_ops(2), record_ops(3)]);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 4 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 2);
    let first_child = &seq_children.active[0];
    assert_eq!(
        first_child.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 2 }]
    );
    assert_eq!(first_child.view_path.len(), 1);
    let second_child = &seq_children.active[1];
    assert_eq!(second_child.operations, &[Operation::Build(3)]);
    assert_eq!(second_child.view_path.len(), 1);

    view2.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(1),
            Operation::Rebuild { from: 1, to: 4 },
            Operation::Teardown(4)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.active.is_empty());
    assert_eq!(seq_children.deleted.len(), 2);
    let (first_child_idx, first_child) = &seq_children.deleted[0];
    assert_eq!(*first_child_idx, 0);
    assert_eq!(
        first_child.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 2 },
            Operation::Teardown(2)
        ]
    );
    let (second_child_idx, second_child) = &seq_children.deleted[1];
    assert_eq!(*second_child_idx, 0);
    assert_eq!(
        second_child.operations,
        &[Operation::Build(3), Operation::Teardown(3)]
    );
}

#[test]
fn normal_messages() {
    let view = sequence(0, vec![record_ops(0), record_ops(1)]);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 2);
    let first_child = &seq_children.active[0];
    let first_path = first_child.view_path.to_vec();

    let second_child = &seq_children.active[1];
    let second_path = second_child.view_path.to_vec();

    let result = view.message(&mut state, &first_path, Box::new(()), &mut ());
    assert_action(result, 0);
    let result = view.message(&mut state, &second_path, Box::new(()), &mut ());
    assert_action(result, 1);

    let view2 = sequence(0, vec![record_ops(2), record_ops(3)]);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();

    let result = view2.message(&mut state, &first_path, Box::new(()), &mut ());
    assert_action(result, 2);
    let result = view2.message(&mut state, &second_path, Box::new(()), &mut ());
    assert_action(result, 3);
}

#[test]
fn stale_messages() {
    let view = sequence(0, vec![record_ops(0)]);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 1);
    let first_child = seq_children.active.first().unwrap();
    let first_path = first_child.view_path.to_vec();

    let result = view.message(&mut state, &first_path, Box::new(()), &mut ());
    assert_action(result, 0);

    let view2 = sequence(0, vec![]);
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();

    let result = view2.message(&mut state, &first_path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));

    let view3 = sequence(0, vec![record_ops(1)]);
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();

    let result = view3.message(&mut state, &first_path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}
