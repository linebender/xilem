// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::{FromDynWidget, Widget, WidgetMut};
use masonry::properties::types::UnitPoint;
use masonry::widgets;
pub use masonry::widgets::ChildAlignment;

use crate::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};
use crate::{Pod, ViewCtx, WidgetView};

/// A widget that lays out its children on top of each other.
/// The children are laid out back to front.
///
/// # Example
///
/// This example shows how to add two text labels on top of each other.
///
/// ```
/// use xilem::WidgetView;
/// use xilem::view::{zstack, label, button};
///
/// fn view<State: 'static>() -> impl WidgetView<State> {
///     zstack((
///         label("Background"),
///         button("Click me", |_| {})
///     ))
/// }
/// ```
pub fn zstack<State, Action, Seq: ZStackSequence<State, Action>>(sequence: Seq) -> ZStack<Seq> {
    ZStack {
        sequence,
        alignment: UnitPoint::CENTER,
    }
}

/// A view container that lays the child widgets on top of each other.
///
/// See [`zstack`] for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ZStack<Seq> {
    sequence: Seq,
    alignment: UnitPoint,
}

impl<Seq> ZStack<Seq> {
    /// Changes the alignment of the children.
    pub fn alignment(mut self, alignment: impl Into<UnitPoint>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

mod hidden {
    use super::ZStackElement;
    use crate::core::AppendVec;

    #[doc(hidden)]
    #[allow(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct ZStackState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<ZStackElement>,
    }
}

use hidden::ZStackState;

impl<Seq> ViewMarker for ZStack<Seq> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for ZStack<Seq>
where
    State: 'static,
    Action: 'static,
    Seq: ZStackSequence<State, Action>,
{
    type Element = Pod<widgets::ZStack>;

    type ViewState = ZStackState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::ZStack::new().with_alignment(self.alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = widget.with_child(child.widget.new_widget, child.alignment);
        }
        let pod = ctx.create_pod(widget);
        (
            pod,
            ZStackState {
                seq_state,
                scratch: elements,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        ZStackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if self.alignment != prev.alignment {
            widgets::ZStack::set_alignment(&mut element, self.alignment);
        }

        let mut splice = ZStackSplice::new(element, scratch);
        self.sequence
            .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        ZStackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut splice = ZStackSplice::new(element, scratch);
        self.sequence
            .seq_teardown(seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        ZStackState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = ZStackSplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

// --- MARK: ZStackExt

/// A trait that extends a [`WidgetView`] with methods to provide parameters for a parent [`ZStack`].
pub trait ZStackExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`ChildAlignment`] to this view.
    /// This allows the view to override the default alignment of the parent [`ZStack`].
    /// This can only be used on views that are direct children of a [`ZStack`].
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

/// A wrapper around a [`WidgetView`], with a specified [`ChildAlignment`].
/// This struct is most often constructed indrectly using [`ZStackExt::alignment`].
pub struct ZStackItem<V, State, Action> {
    view: V,
    alignment: ChildAlignment,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Constructs a new `ZStackItem`.
/// See also [`ZStackExt::alignment`], for constructing a `ZStackItem` from an existing view.
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

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (ZStackElement::new(pod.erased(), self.alignment), state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        {
            if self.alignment != prev.alignment {
                widgets::ZStack::update_child_alignment(
                    &mut element.parent,
                    element.idx,
                    self.alignment,
                );
            }
            let mut child = widgets::ZStack::child_mut(&mut element.parent, element.idx)
                .expect("ZStackWrapper always has a widget child");
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut child = widgets::ZStack::child_mut(&mut element.parent, element.idx)
            .expect("ZStackWrapper always has a widget child");
        self.view
            .teardown(view_state, ctx, child.downcast(), app_state);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::ZStack::child_mut(&mut element.parent, element.idx)
            .expect("ZStackWrapper always has a corresponding child");
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}

// --- MARK: ZStackElement

/// A struct implementing [`ViewElement`] for a `ZStack`.
pub struct ZStackElement {
    widget: Pod<dyn Widget>,
    alignment: ChildAlignment,
}

/// A mutable version of `ZStackElement`.
pub struct ZStackElementMut<'w> {
    parent: WidgetMut<'w, widgets::ZStack>,
    idx: usize,
}

impl ZStackElement {
    fn new(widget: Pod<dyn Widget>, alignment: ChildAlignment) -> Self {
        Self { widget, alignment }
    }
}

impl ViewElement for ZStackElement {
    type Mut<'a> = ZStackElementMut<'a>;
}

impl SuperElement<Self, ViewCtx> for ZStackElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
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

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for ZStackElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self::new(child.erased(), ChildAlignment::ParentAligned)
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = widgets::ZStack::child_mut(&mut this.parent, this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// MARK: Sequence

/// A trait implementing `ViewSequence` for `ZStackElement`.
pub trait ZStackSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

impl<Seq, State, Action> ZStackSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

// MARK: Splice

/// An implementation of [`ElementSplice`] for `ZStackElement`.
pub struct ZStackSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::ZStack>,
    scratch: &'s mut AppendVec<ZStackElement>,
}

impl<'w, 's> ZStackSplice<'w, 's> {
    fn new(
        element: WidgetMut<'w, widgets::ZStack>,
        scratch: &'s mut AppendVec<ZStackElement>,
    ) -> Self {
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}

impl ElementSplice<ZStackElement> for ZStackSplice<'_, '_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<ZStackElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            widgets::ZStack::insert_child(
                &mut self.element,
                element.widget.new_widget,
                element.alignment,
            );
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: ZStackElement) {
        widgets::ZStack::insert_child(
            &mut self.element,
            element.widget.new_widget,
            element.alignment,
        );
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, ZStackElement>) -> R) -> R {
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

    fn index(&self) -> usize {
        self.idx
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, ZStackElement>) -> R) -> R {
        let ret = {
            let child = ZStackElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::ZStack::remove_child(&mut self.element, self.idx);
        ret
    }
}
