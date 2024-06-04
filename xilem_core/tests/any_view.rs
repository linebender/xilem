// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests that [`AnyView`] has the correct routing behaviour

use xilem_core::{AnyView, MessageResult, View};

mod common;
use common::*;

type AnyNoopView = dyn AnyView<(), Action, TestCx, TestElement>;

#[test]
fn any_view_routes_to_one_view() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCx(vec![]);
    let (element, mut state) = view.build(&mut ctx);
    let result = view.message(&mut state, &element.view_path, Box::new(()), &mut ());
    assert_action(result, 0);
}

#[test]
fn any_view_routes_after_rebuild() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view.build(&mut ctx);
    let path = element.view_path.clone();
    let view2: Box<AnyNoopView> = Box::new(OperationView::<0>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert_action(result, 1);
}

#[test]
fn any_view_no_route_after_stale() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view.build(&mut ctx);
    let path = element.view_path.clone();
    let view2: Box<AnyNoopView> = Box::new(OperationView::<1>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);
    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}

#[test]
fn any_view_no_route_after_stale_then_same_type() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCx(vec![]);
    let (mut element, mut state) = view.build(&mut ctx);
    let path = element.view_path.clone();
    let view2: Box<AnyNoopView> = Box::new(OperationView::<1>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element);

    let view3: Box<AnyNoopView> = Box::new(OperationView::<0>(2));
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element);
    let result = view3.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}
