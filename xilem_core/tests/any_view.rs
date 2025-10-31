// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests that [`AnyView`] has the correct routing behaviour

use xilem_core::{AnyView, DynMessage, MessageResult, View};

mod common;
use common::*;

type AnyNoopView = dyn AnyView<(), Action, TestCtx, TestElement>;

#[test]
fn messages_to_inner_view() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, ());
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);
    ctx.with_message_context(element.view_path.clone(), DynMessage::new(()), |ctx| {
        let result = view.message(&mut state, ctx, &mut element, ());
        assert_action(result, 0);
    });
}

#[test]
fn message_after_rebuild() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, ());
    ctx.assert_empty();
    let path = element.view_path.clone();

    let view2: Box<AnyNoopView> = Box::new(OperationView::<0>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element, ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 1 }]
    );

    ctx.with_message_context(path, DynMessage::new(()), |ctx| {
        let result = view2.message(&mut state, ctx, &mut element, ());
        assert_action(result, 1);
    });
}

#[test]
fn no_message_after_stale() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, ());
    ctx.assert_empty();
    let path = element.view_path.clone();

    let view2: Box<AnyNoopView> = Box::new(OperationView::<1>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element, ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1)
        ]
    );

    ctx.with_message_context(path, DynMessage::new(()), |ctx| {
        let result = view2.message(&mut state, ctx, &mut element, ());
        assert!(matches!(result, MessageResult::Stale));
    });
}

#[test]
fn no_message_after_stale_then_same_type() {
    let view: Box<AnyNoopView> = Box::new(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, ());
    ctx.assert_empty();
    let path = element.view_path.clone();

    let view2: Box<AnyNoopView> = Box::new(OperationView::<1>(1));
    view2.rebuild(&view, &mut state, &mut ctx, &mut element, ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1)
        ]
    );

    let view3: Box<AnyNoopView> = Box::new(OperationView::<0>(2));
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element, ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1),
            Operation::Teardown(1),
            Operation::Replace(2)
        ]
    );

    ctx.with_message_context(path, DynMessage::new(()), |ctx| {
        let result = view3.message(&mut state, ctx, &mut element, ());
        assert!(matches!(result, MessageResult::Stale));
    });
}
