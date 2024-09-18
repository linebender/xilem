// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests for the behaviour of [`OneOf2<A, B>`] where `A` and `B` is a view.
//!
//! This is an integration test so that it can use the infrastructure in [`common`].

use xilem_core::{
    one_of::{OneOf, OneOf2, OneOfCtx, PhantomElementCtx},
    MessageResult, Mut, View, ViewId,
};

mod common;
use common::*;

fn record_ops_0(id: u32) -> OperationView<0> {
    OperationView(id)
}

fn record_ops_1(id: u32) -> OperationView<1> {
    OperationView(id)
}

impl PhantomElementCtx for TestCtx {
    type PhantomElement = TestElement;
}

impl
    OneOfCtx<
        TestElement,
        TestElement,
        TestElement,
        TestElement,
        TestElement,
        TestElement,
        TestElement,
        TestElement,
        TestElement,
    > for TestCtx
{
    type OneOfElement = TestElement;

    fn upcast_one_of_element(
        &mut self,
        elem: OneOf<
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
        >,
    ) -> Self::OneOfElement {
        match elem {
            OneOf::A(e) => e,
            OneOf::B(e) => e,
            _ => unreachable!(),
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
        new_elem: OneOf<
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
            TestElement,
        >,
    ) {
        match new_elem {
            OneOf::A(new_elem) | OneOf::B(new_elem) => {
                assert_eq!(new_elem.operations.len(), 1);
                let Some(Operation::Build(new_id)) = new_elem.operations.first() else {
                    unreachable!()
                };
                elem_mut.operations.push(Operation::Replace(*new_id));
                elem_mut.view_path = new_elem.view_path;
                elem_mut.children = new_elem.children;
            }
            _ => unreachable!(),
        }
    }

    fn with_downcast_a(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, TestElement>),
    ) {
        f(elem);
    }

    fn with_downcast_b(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, TestElement>),
    ) {
        f(elem);
    }

    // when one of the following would be invoked, it would be an error in the impl of `OneOfN`
    fn with_downcast_c(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_d(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_e(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_f(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_g(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_h(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }

    fn with_downcast_i(
        _elem: &mut Mut<'_, Self::OneOfElement>,
        _f: impl FnOnce(Mut<'_, <Self as PhantomElementCtx>::PhantomElement>),
    ) {
        unreachable!()
    }
}

#[test]
/// As the view types can change, a view id/generation is necessary
fn one_of_path() {
    let view1: OneOf2<OperationView<0>, OperationView<1>> = OneOf2::A(record_ops_0(0));
    let mut ctx = TestCtx::default();
    let (element, _state) = view1.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.view_path.len(), 1);
    assert_eq!(element.view_path[0], ViewId::new(0));
}

#[test]
/// A rebuild with the same type/variant should be (almost) equivalent to just using the view itself
fn one_of_same_type_rebuild() {
    let view1: OneOf2<OperationView<0>, OperationView<1>> = OneOf2::A(record_ops_0(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();

    let view2 = OneOf2::A(record_ops_0(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(element.view_path[0], ViewId::new(0));
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 1 }]
    );
}

#[test]
/// A type change (via different variant) changes the view path and tears down the old view
fn one_of_type_change_rebuild() {
    let view1 = OneOf2::A(record_ops_0(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();

    let view2 = OneOf2::B(record_ops_1(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(element.view_path[0], ViewId::new(1));
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1),
        ]
    );
}

#[test]
/// `OneOf2` should successfully allow the child to teardown
fn one_of_passthrough_teardown() {
    let view1: OneOf2<OperationView<0>, OperationView<1>> = OneOf2::A(record_ops_0(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);

    view1.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(element.view_path[0], ViewId::new(0));
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Teardown(0)]
    );
}

#[test]
fn one_of_passthrough_message() {
    let view1: OneOf2<OperationView<0>, OperationView<1>> = OneOf2::A(record_ops_0(0));
    let mut ctx = TestCtx::default();
    let (element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);

    let result = view1.message(&mut state, &element.view_path, Box::new(()), &mut ());
    assert_action(result, 0);
}

#[test]
fn one_of_no_message_after_stale() {
    let view1 = OneOf2::A(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();
    let path = element.view_path.clone();

    let view2 = OneOf2::B(OperationView::<1>(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1)
        ]
    );

    let result = view2.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}

#[test]
fn one_of_no_message_after_stale_then_same_type() {
    let view1 = OneOf2::A(OperationView::<0>(0));
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx);
    ctx.assert_empty();
    let path = element.view_path.clone();

    let view2 = OneOf2::B(OperationView::<1>(1));
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Teardown(0),
            Operation::Replace(1)
        ]
    );

    let view3 = OneOf2::A(OperationView::<0>(2));
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element);
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

    let result = view3.message(&mut state, &path, Box::new(()), &mut ());
    assert!(matches!(result, MessageResult::Stale(_)));
}
