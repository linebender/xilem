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
    let mut ctx = TestCx(vec![]);
    let (element, _state) = view.build(&mut ctx);
    assert!(element.children.unwrap().active.is_empty());
}

#[test]
fn single_one_element() {
    let view = sequence(0, record_ops(0));
    let mut ctx = TestCx(vec![]);
    let (element, _state) = view.build(&mut ctx);
    assert_eq!(element.children.unwrap().active.len(), 1);
}
