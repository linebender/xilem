// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};
use crate::{Pod, PropertyTuple as _, ViewCtx};
use masonry::core::{FromDynWidget, Widget, WidgetMut};
use masonry::{
    properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding},
    widgets,
};

use crate::style::Style;

/// An `IndexedStack` displays one of several children elements at a time.
///
/// This is useful for switching between multiple views while keeping
/// state loaded, such as in a tab stack.
///
/// The indexed stack acts as a simple container around the active child.
/// If there is no active child, it acts like a leaf node, and takes up
/// the minimum space.
///
/// # Example
/// ```
/// use xilem::view::{
///     button, indexed_stack, label, flex
/// };
///
/// #[derive(Default)]
/// struct State {
///     tab: usize,
/// }
///
/// let mut state = State::default();
///
/// indexed_stack(
///     (   
///         flex((
///             label("Tab A"),
///             button("Move to tab B", |state: &mut State| state.tab = 1)
///         )),
///         flex((
///             label("Tab B"),
///             button("Move to tab A", |state: &mut State| state.tab = 0)
///         )),
///     ),
/// )
/// .active(state.tab);
/// ```
pub fn indexed_stack<State, Action, Seq: IndexedStackSequence<State, Action>>(
    sequence: Seq,
) -> IndexedStack<Seq, State, Action> {
    IndexedStack {
        sequence,
        active_child: 0,
        properties: IndexedStackProps::default(),
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`indexed_stack`] from a sequence.
///
/// See `indexed_stack` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct IndexedStack<Seq, State, Action = ()> {
    sequence: Seq,
    active_child: usize,
    properties: IndexedStackProps,

    /// Used to associate the State and Action in the call to `.indexed_stack()` with the State and Action
    /// used in the View implementation, to allow inference to flow backwards, allowing State and
    /// Action to be inferred properly.
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> IndexedStack<Seq, State, Action> {
    /// Set the active item for this stack.
    #[track_caller]
    pub fn active(mut self, active: usize) -> Self {
        // TODO: validate this against the sequence. Currently,
        // the sequence has no way to get the length.
        self.active_child = active;
        self
    }
}

impl<Seq, S, A> Style for IndexedStack<Seq, S, A> {
    type Props = IndexedStackProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    pub IndexedStackProps;
    IndexedStack<Seq, S, A>;

    Background, 0;
    BorderColor, 1;
    BorderWidth, 2;
    CornerRadius, 3;
    Padding, 4;
);

mod hidden {
    use super::IndexedStackElement;
    use crate::core::AppendVec;

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct IndexedStackState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<IndexedStackElement>,
    }
}

use hidden::IndexedStackState;

impl<Seq, State, Action> ViewMarker for IndexedStack<Seq, State, Action> {}

impl<State, Action, Seq> View<State, Action, ViewCtx> for IndexedStack<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: IndexedStackSequence<State, Action>,
{
    type Element = Pod<widgets::IndexedStack>;

    type ViewState = IndexedStackState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::IndexedStack::new();
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for element in elements.drain() {
            widget = widget.with_child(element.child.new_widget);
        }
        widget = widget.with_active_child(self.active_child);
        let mut pod = ctx.create_pod(widget);
        pod.new_widget.properties = self.properties.build_properties();
        (
            pod,
            IndexedStackState {
                seq_state,
                scratch: elements,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        IndexedStackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        {
            let mut splice = IndexedStackSplice::new(element.reborrow_mut(), scratch);
            self.sequence
                .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
            debug_assert!(scratch.is_empty());
        }

        // set the active child after updating the sequence to
        // ensure the index remains consistent with the children list
        if self.active_child != element.widget.active_child_index() {
            widgets::IndexedStack::set_active_child(&mut element, self.active_child);
        }
    }

    fn teardown(
        &self,
        IndexedStackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut splice = IndexedStackSplice::new(element, scratch);
        self.sequence
            .seq_teardown(seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        IndexedStackState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = IndexedStackSplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

// Used to become a reference form for editing. It's provided to rebuild and teardown.
impl ViewElement for IndexedStackElement {
    type Mut<'w> = IndexedStackElementMut<'w>;
}

// Used to allow the item to be used as a generic item in ViewSequence.
impl SuperElement<Self, ViewCtx> for IndexedStackElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = IndexedStackElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for IndexedStackElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self {
            child: child.erased(),
        }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::IndexedStack::child_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// Used for building and rebuilding the ViewSequence
impl ElementSplice<IndexedStackElement> for IndexedStackSplice<'_, '_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<IndexedStackElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            widgets::IndexedStack::insert_child(
                &mut self.element,
                self.idx,
                element.child.new_widget,
            );
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: IndexedStackElement) {
        widgets::IndexedStack::insert_child(&mut self.element, self.idx, element.child.new_widget);
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, IndexedStackElement>) -> R) -> R {
        let child = IndexedStackElementMut {
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

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, IndexedStackElement>) -> R) -> R {
        let ret = {
            let child = IndexedStackElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::IndexedStack::remove_child(&mut self.element, self.idx);
        ret
    }
}

/// `IndexedStackSequence` is what allows an input to the indexed stack that contains all the stack elements.
pub trait IndexedStackSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, IndexedStackElement>
{
}

impl<Seq, State, Action> IndexedStackSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, IndexedStackElement>
{
}
/// A child widget within a [`IndexedStack`] view.
pub struct IndexedStackElement {
    /// The child widget.
    child: Pod<dyn Widget>,
}

/// A mutable reference to a [`IndexedStackElement`], used internally by Xilem traits.
pub struct IndexedStackElementMut<'w> {
    parent: WidgetMut<'w, widgets::IndexedStack>,
    idx: usize,
}

// Used for manipulating the ViewSequence.
struct IndexedStackSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::IndexedStack>,
    scratch: &'s mut AppendVec<IndexedStackElement>,
}

impl<'w, 's> IndexedStackSplice<'w, 's> {
    fn new(
        element: WidgetMut<'w, widgets::IndexedStack>,
        scratch: &'s mut AppendVec<IndexedStackElement>,
    ) -> Self {
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}
