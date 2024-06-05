// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod common;
use common::*;
use xilem_core::View;

fn record_ops(id: u32) -> OperationView<0> {
    OperationView(id)
}

#[test]
fn unit_no_elements() {
    let view = sequence(0, ());
    let mut ctx = TestCx::default();
    let (element, _state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert!(element.children.unwrap().active.is_empty());
}

/// The sequence (item,) should pass through all methods to the child
#[test]
fn one_element_passthrough() {
    let view = sequence(1, (record_ops(0),));
    let mut ctx = TestCx::default();
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
        "The single item tuple ViewSequence shouldn't add to the view path"
    );

    let view2 = sequence(3, (record_ops(2),));
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

/// The sequence (item, item) should pass through all methods to the children
#[test]
fn two_element_passthrough() {
    let view = sequence(2, (record_ops(0), record_ops(1)));
    let mut ctx = TestCx::default();
    let (mut element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(2)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 2);
    let first_child = &seq_children.active[0];
    assert_eq!(first_child.operations, &[Operation::Build(0)]);
    assert_eq!(first_child.view_path.len(), 1);
    let second_child = &seq_children.active[1];
    assert_eq!(second_child.operations, &[Operation::Build(1)]);
    assert_eq!(second_child.view_path.len(), 1);

    let view2 = sequence(5, (record_ops(3), record_ops(4)));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(2), Operation::Rebuild { from: 2, to: 5 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 2);
    let first_child = &seq_children.active[0];
    assert_eq!(
        first_child.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 3 }]
    );
    let second_child = &seq_children.active[1];
    assert_eq!(
        second_child.operations,
        &[Operation::Build(1), Operation::Rebuild { from: 1, to: 4 }]
    );

    view2.teardown(&mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[
            Operation::Build(2),
            Operation::Rebuild { from: 2, to: 5 },
            Operation::Teardown(5)
        ]
    );

    let seq_children = element.children.as_ref().unwrap();
    // It was removed from the parent sequence when tearing down
    assert_eq!(seq_children.active.len(), 0);
    assert_eq!(seq_children.deleted.len(), 2);
    let (first_child_idx, first_child) = &seq_children.deleted[0];
    assert_eq!(*first_child_idx, 0);
    assert_eq!(
        first_child.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 3 },
            Operation::Teardown(3)
        ]
    );
    let (second_child_idx, second_child) = &seq_children.deleted[1];
    // At the time of being deleted, this was effectively the item at index 0
    assert_eq!(*second_child_idx, 0);
    assert_eq!(
        second_child.operations,
        &[
            Operation::Build(1),
            Operation::Rebuild { from: 1, to: 4 },
            Operation::Teardown(4)
        ]
    );
}

/// The sequence (item, item) should pass through all methods to the children
#[test]
fn two_element_message() {
    let view = sequence(2, (record_ops(0), record_ops(1)));
    let mut ctx = TestCx::default();
    let (element, mut state) = view.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(2)]);
    assert_eq!(element.view_path, &[]);

    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 2);
    let first_child = &seq_children.active[0];
    assert_eq!(first_child.operations, &[Operation::Build(0)]);
    let first_path = first_child.view_path.to_vec();
    let second_child = &seq_children.active[1];
    assert_eq!(second_child.operations, &[Operation::Build(1)]);
    let second_path = second_child.view_path.to_vec();

    let result = view.message(&mut state, &first_path, Box::new(()), &mut ());
    assert_action(result, 0);

    let result = view.message(&mut state, &second_path, Box::new(()), &mut ());
    assert_action(result, 1);
}

// We don't test higher tuples, because these all use the same implementation
