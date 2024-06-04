// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests for the behaviour of [`Arc<V>`] where `V` is a view.
//!
//! Also has some tests for [`Box<V>`], for which there is no special behaviour
//!
//! This is an integration test so that it can use the infrastructure in [`common`].

use std::sync::Arc;
use xilem_core::{MessageResult, View};

mod common;
use common::*;

fn record_ops(id: u32) -> OperationView<0> {
    OperationView(id)
}

#[test]
/// The Arc view shouldn't impact the view path
fn arc_no_path() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (element, ()) = view1.build(&mut ctx);
    assert!(element.view_path.is_empty());
}

#[test]
fn same_arc_skip_rebuild() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);
    let view2 = Arc::clone(&view1);
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    assert_eq!(element.operations, &[Operation::Build(0)]);
}

#[test]
/// If use a different Arc, a rebuild should happen
fn new_arc_rebuild() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);
    let view2 = Arc::new(record_ops(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 1 }]
    );
}

#[test]
/// If use a different Arc, a rebuild should happen
fn new_arc_rebuild_same_value() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);
    let view2 = Arc::new(record_ops(0));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 0 }]
    );
}

#[test]
/// Arc should successfully allow the child to teardown
fn arc_passthrough_teardown() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);

    view1.teardown(&mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Teardown(0)]
    );
}

#[test]
fn arc_passthrough_message() {
    let view1 = Arc::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);

    let result = view1.message(&mut state, &element.view_path, Box::new(()), &mut ());
    assert_action(result, 0);
}

/// --- MARK: Box tests ---
#[test]
/// The Box view shouldn't impact the view path
fn box_no_path() {
    let view1 = Box::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (element, ()) = view1.build(&mut ctx);
    assert!(element.view_path.is_empty());
}

#[test]
/// The Box view should always rebuild
fn box_passthrough_rebuild() {
    let view1 = Box::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);
    let view2 = Box::new(record_ops(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 1 }]
    );
}

#[test]
/// The Box view should always rebuild
fn box_passthrough_rebuild_same_value() {
    let view1 = Box::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);
    let view2 = Box::new(record_ops(0));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 0 }]
    );
}

#[test]
fn box_passthrough_teardown() {
    let view1 = Box::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);

    view1.teardown(&mut state, &mut ctx, &mut element);
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Teardown(0)]
    );
}

#[test]
fn box_passthrough_message() {
    let view1 = Box::new(record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (element, mut state) = view1.build(&mut ctx);
    assert_eq!(element.operations, &[Operation::Build(0)]);

    let result = view1.message(&mut state, &element.view_path, Box::new(()), &mut ());
    let MessageResult::Action(inner) = result else {
        panic!()
    };
    assert_eq!(inner.id, 0);
}
