// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests for [`SequenceView`] with vectors.

mod common;
use common::*;
use xilem_core::View;

fn record_ops(id: u32) -> OperationView<0> {
    OperationView(id)
}

#[test]
fn vec_in_tuple() {
    let view = sequence(3, (record_ops(0), vec![record_ops(1), record_ops(2)]));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, ());
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(3)]);
    assert_eq!(element.view_path, &[]);

    // Children after build have expected shape
    let seq_children = element.children.as_ref().unwrap();
    assert!(seq_children.deleted.is_empty());
    assert_eq!(seq_children.active.len(), 3);

    // The tuple child
    let child = &seq_children.active[0];
    assert_eq!(child.operations, &[Operation::Build(0)]);
    assert_eq!(child.view_path.len(), 1);

    // The Vec children
    let child = &seq_children.active[1];
    assert_eq!(child.operations, &[Operation::Build(1)]);
    // (Tuple then vec)
    assert_eq!(child.view_path.len(), 2);
    let child = &seq_children.active[2];
    assert_eq!(child.operations, &[Operation::Build(2)]);
    assert_eq!(child.view_path.len(), 2);

    // Rebuild
    let view2 = sequence(5, (record_ops(4), vec![]));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element, ());
    ctx.assert_empty();

    assert_eq!(
        element.operations,
        &[Operation::Build(3), Operation::Rebuild { from: 3, to: 5 }]
    );

    let seq_children = element.children.as_ref().unwrap();
    // Teardowns from deleted elements in Vec
    assert_eq!(seq_children.deleted.len(), 2);

    let (child_idx, child) = &seq_children.deleted[0];
    assert_eq!(*child_idx, 1);
    assert_eq!(child.view_path.len(), 2);
    assert_eq!(
        child.operations,
        &[Operation::Build(1), Operation::Teardown(1)]
    );
    let (child_idx, child) = &seq_children.deleted[1];
    // N.B. we delete the item with the index we see it with (i.e. after the previous item has been deleted)
    assert_eq!(*child_idx, 1);
    assert_eq!(child.view_path.len(), 2);
    assert_eq!(
        child.operations,
        &[Operation::Build(2), Operation::Teardown(2)]
    );

    assert_eq!(seq_children.active.len(), 1);

    let child = &seq_children.active[0];
    assert_eq!(
        child.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 4 }]
    );
    assert_eq!(child.view_path.len(), 1);
}
