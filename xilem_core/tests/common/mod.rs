// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_core::*;

pub(crate) struct TestCx(pub Vec<ViewId>);

impl ViewPathTracker for TestCx {
    fn push_id(&mut self, id: ViewId) {
        self.0.push(id)
    }
    fn pop_id(&mut self) {
        self.0.pop();
    }
    fn view_path(&mut self) -> &[ViewId] {
        &self.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum Operation {
    Build(u32),
    Rebuild { from: u32, to: u32 },
    Teardown(u32),
    Replace(u32),
}

#[derive(Clone)]
pub(crate) struct TestElement {
    pub operations: Vec<Operation>,
    pub view_path: Vec<ViewId>,
    /// The child sequence, if applicable
    ///
    /// This avoids having to create more element types
    pub sequences: Option<SeqTracker>,
}
impl ViewElement for TestElement {
    type Mut<'a> = &'a mut Self;
}

pub struct OperationView<const N: u32>(pub u32);

pub struct Action {
    pub id: u32,
    _priv: (),
}

impl<const N: u32> View<(), Action, TestCx> for OperationView<N> {
    type Element = TestElement;

    type ViewState = ();

    fn build(&self, ctx: &mut TestCx) -> (Self::Element, Self::ViewState) {
        (
            TestElement {
                operations: vec![Operation::Build(self.0)],
                view_path: ctx.view_path().to_vec(),
                sequences: None,
            },
            (),
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        ctx: &mut TestCx,
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        assert_eq!(&*element.view_path, ctx.view_path());
        element.operations.push(Operation::Rebuild {
            from: prev.0,
            to: self.0,
        });
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut TestCx,
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        assert_eq!(&*element.view_path, ctx.view_path());
        element.operations.push(Operation::Teardown(self.0));
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[ViewId],
        _: DynMessage,
        _: &mut (),
    ) -> MessageResult<Action> {
        // If we get an `Action` value, we know it came from here
        MessageResult::Action(Action {
            _priv: (),
            id: self.0,
        })
    }
}

impl SuperElement<TestElement> for TestElement {
    fn upcast(child: TestElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        this: Self::Mut<'_>,
        f: impl FnOnce(<TestElement as ViewElement>::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = f(this);
        (this, ret)
    }
}

impl AnyElement<TestElement> for TestElement {
    fn replace_inner(this: Self::Mut<'_>, child: TestElement) -> Self::Mut<'_> {
        assert_eq!(child.operations.len(), 1);
        let Operation::Build(child_id) = child.operations.first().unwrap() else {
            panic!()
        };
        assert_ne!(child.view_path, this.view_path);
        this.operations.push(Operation::Replace(*child_id));
        this.view_path = child.view_path;
        if let Some((mut new_seq, old_seq)) = child.sequences.zip(this.sequences.as_mut()) {
            new_seq.deleted.extend(old_seq.deleted.iter().cloned());
            new_seq
                .deleted
                .extend(old_seq.active.iter().cloned().enumerate());
            *old_seq = new_seq;
        }
        this
    }
}

#[derive(Clone)]
pub struct SeqTracker {
    active: Vec<TestElement>,
    deleted: Vec<(usize, TestElement)>,
}

#[track_caller]
pub fn assert_action(result: MessageResult<Action>, id: u32) {
    let MessageResult::Action(inner) = result else {
        panic!()
    };
    assert_eq!(inner.id, id);
}
