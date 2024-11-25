// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::{
    core::{
        AppendVec, DynMessage, ElementSplice, Mut, SuperElement, View, ViewElement, ViewMarker,
        ViewSequence,
    },
    Pod, ViewCtx, WidgetView,
};
use masonry::{
    widget::{self, Alignment, ChildAlignment, WidgetMut},
    Widget,
};
use xilem_core::{MessageResult, ViewId};

/// A widget that lays out its children on top of each other.
/// The children are laid out back to front.
///
/// # Example
///
/// This example shows how to add two text labels on top of each other.
///
/// ```
/// use xilem::WidgetView;
/// use xilem::view::{zstack, label};
///
/// fn view() -> impl WidgetView<()> {
///     zstack::<(), (), _>((
///         label("Background"),
///         label("Foreground")
///     ))
/// }
/// ```
pub fn zstack<State, Action, Seq: ZStackSequence<State, Action>>(sequence: Seq) -> ZStack<Seq> {
    ZStack {
        sequence,
        alignment: Alignment::default(),
    }
}

/// A view container that lays the child widgets on top of each other.
///
/// See [`zstack`] for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ZStack<Seq> {
    sequence: Seq,
    alignment: Alignment,
}

impl<Seq> ZStack<Seq> {
    /// Changes the alignment of the children.
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

impl<Seq> ViewMarker for ZStack<Seq> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for ZStack<Seq>
where
    State: 'static,
    Action: 'static,
    Seq: ZStackSequence<State, Action>,
{
    type Element = Pod<widget::ZStack>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::ZStack::new().with_alignment(self.alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = widget.with_child_pod(child.widget.inner, child.alignment);
        }
        (ctx.new_pod(widget), seq_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if self.alignment != prev.alignment {
            widget::ZStack::set_alignment(&mut element, self.alignment);
        }

        let mut splice = StackSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.is_empty());
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        let mut splice = StackSplice::new(element);
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

// --- MARK: ZStackExt ---

pub trait ZStackExt<State, Action>: WidgetView<State, Action> {
    fn alignment(self, alignment: impl Into<ChildAlignment>) -> ZStackItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        zstack_item(self, alignment)
    }
}

impl<State, Action, V: WidgetView<State, Action>> ZStackExt<State, Action> for V {}

pub struct ZStackItem<V, State, Action> {
    view: V,
    alignment: ChildAlignment,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn zstack_item<V, State, Action>(
    view: V,
    alignment: impl Into<ChildAlignment>,
) -> ZStackItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    ZStackItem {
        view,
        alignment: alignment.into(),
        phantom: PhantomData,
    }
}

impl<V, State, Action> ViewMarker for ZStackItem<V, State, Action> {}

impl<State, Action, V> View<State, Action, ViewCtx> for ZStackItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = ZStackElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx);
        (
            ZStackElement::new(ctx.boxed_pod(pod), self.alignment),
            state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        {
            if self.alignment != prev.alignment {
                widget::ZStack::update_child_alignment(
                    &mut element.parent,
                    element.idx,
                    self.alignment,
                );
            }
            let mut child = widget::ZStack::child_mut(&mut element.parent, element.idx)
                .expect("ZStackWrapper always has a widget child");
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast());
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let mut child = widget::ZStack::child_mut(&mut element.parent, element.idx)
            .expect("ZStackWrapper always has a widget child");
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}

// --- MARK: ZStackElement ---
pub struct ZStackElement {
    widget: Pod<Box<dyn Widget>>,
    alignment: ChildAlignment,
}

pub struct ZStackElementMut<'w> {
    parent: WidgetMut<'w, widget::ZStack>,
    idx: usize,
}

impl ZStackElement {
    fn new(widget: Pod<Box<dyn Widget>>, alignment: ChildAlignment) -> Self {
        Self { widget, alignment }
    }
}

impl ViewElement for ZStackElement {
    type Mut<'a> = ZStackElementMut<'a>;
}

impl SuperElement<ZStackElement, ViewCtx> for ZStackElement {
    fn upcast(_ctx: &mut ViewCtx, child: ZStackElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<Self>,
        f: impl FnOnce(Mut<ZStackElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = ZStackElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for ZStackElement {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        ZStackElement::new(ctx.boxed_pod(child), ChildAlignment::ParentAligned)
    }

    fn with_downcast_val<R>(
        mut this: Mut<Self>,
        f: impl FnOnce(Mut<Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = widget::ZStack::child_mut(&mut this.parent, this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// MARK: Sequence
pub trait ZStackSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

impl<Seq, State, Action> ZStackSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

// MARK: Splice

pub struct StackSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::ZStack>,
    scratch: AppendVec<ZStackElement>,
}

impl<'w> StackSplice<'w> {
    fn new(element: WidgetMut<'w, widget::ZStack>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ElementSplice<ZStackElement> for StackSplice<'_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<ZStackElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            widget::ZStack::insert_child_pod(
                &mut self.element,
                element.widget.inner,
                element.alignment,
            );
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: ZStackElement) {
        widget::ZStack::insert_child_pod(
            &mut self.element,
            element.widget.inner,
            element.alignment,
        );
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<ZStackElement>) -> R) -> R {
        let child = ZStackElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<ZStackElement>) -> R) -> R {
        let ret = {
            let child = ZStackElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widget::ZStack::remove_child(&mut self.element, self.idx);
        ret
    }
}
