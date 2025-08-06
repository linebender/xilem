// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::style::Style;

use masonry::core::{Axis, FromDynWidget, Widget, WidgetMut};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use masonry::widgets::{self};

use crate::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};
use crate::{Pod, PropertyTuple as _, ViewCtx, WidgetView};

/// A linear container.

// TODO - Add example

pub fn stack<State, Action, Seq: StackSequence<State, Action>>(
    sequence: Seq,
) -> Stack<Seq, State, Action> {
    Stack {
        sequence,
        axis: Axis::Vertical,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        gap: masonry::theme::DEFAULT_GAP,
        properties: StackProps::default(),
        phantom: PhantomData,
    }
}

/// A layout where the children are laid out in a row.
///
/// This is equivalent to [`stack`] with a pre-applied horizontal
/// [`direction`](Stack::direction).
pub fn stack_row<State, Action, Seq: StackSequence<State, Action>>(
    sequence: Seq,
) -> Stack<Seq, State, Action> {
    stack(sequence).direction(Axis::Horizontal)
}

/// The [`View`] created by [`stack`] from a sequence.
///
/// See `stack` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Stack<Seq, State, Action = ()> {
    sequence: Seq,
    axis: Axis,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    gap: f64,
    properties: StackProps,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> Stack<Seq, State, Action> {
    /// Set the flex direction (see [`Axis`]).
    pub fn direction(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    /// Set the children's [`CrossAxisAlignment`].
    pub fn cross_axis_alignment(mut self, axis: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = axis;
        self
    }
    /// Set the children's [`MainAxisAlignment`].
    pub fn main_axis_alignment(mut self, axis: MainAxisAlignment) -> Self {
        self.main_axis_alignment = axis;
        self
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Similar to the css [gap] property.
    ///
    /// Leave unset to use the default spacing which is [`DEFAULT_GAP`].
    ///
    /// # Panics
    ///
    /// If `gap` is not a non-negative finite value.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    /// [`DEFAULT_GAP`]: masonry::theme::DEFAULT_GAP
    #[track_caller]
    pub fn gap(mut self, gap: f64) -> Self {
        if gap.is_finite() && gap >= 0.0 {
            self.gap = gap;
        } else {
            // TODO: Don't panic here, for future editor scenarios.
            panic!("Invalid `gap` {gap}, expected a non-negative finite value.")
        }
        self
    }
}

impl<Seq, S, A> Style for Stack<Seq, S, A> {
    type Props = StackProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    pub StackProps;
    Stack<Seq, S, A>;

    Background, 0;
    BorderColor, 1;
    BorderWidth, 2;
    CornerRadius, 3;
    Padding, 4;
);

mod hidden {
    use crate::core::AppendVec;
    use crate::view::StackElement;

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct StackState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<StackElement>,
    }
}

use hidden::StackState;

impl<Seq, State, Action> ViewMarker for Stack<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Stack<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: StackSequence<State, Action>,
{
    type Element = Pod<widgets::Stack>;

    type ViewState = StackState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Stack::for_axis(self.axis)
            .with_gap(self.gap)
            .cross_axis_alignment(self.cross_axis_alignment)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = widget.with_child(child.child.new_widget, child.alignment);
        }
        let mut pod = ctx.create_pod(widget);
        pod.new_widget.properties = self.properties.build_properties();
        (
            pod,
            StackState {
                seq_state,
                scratch: elements,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        StackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        if prev.axis != self.axis {
            widgets::Stack::set_direction(&mut element, self.axis);
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            widgets::Stack::set_cross_axis_alignment(&mut element, self.cross_axis_alignment);
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            widgets::Stack::set_main_axis_alignment(&mut element, self.main_axis_alignment);
        }
        if prev.gap != self.gap {
            widgets::Stack::set_gap(&mut element, self.gap);
        }
        let mut splice = StackSplice::new(element, scratch);
        self.sequence
            .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        StackState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut splice = StackSplice::new(element, scratch);
        self.sequence
            .seq_teardown(seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        StackState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = StackSplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

/// A child element of a [`Stack`] view.
pub struct StackElement {
    /// Child widget.
    child: Pod<dyn Widget>,
    alignment: Option<CrossAxisAlignment>,
}

/// A mutable reference to a [`StackElement`], used internally by Xilem traits.
pub struct StackElementMut<'w> {
    parent: WidgetMut<'w, widgets::Stack>,
    idx: usize,
}

struct StackSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::Stack>,
    scratch: &'s mut AppendVec<StackElement>,
}

impl<'w, 's> StackSplice<'w, 's> {
    fn new(
        element: WidgetMut<'w, widgets::Stack>,
        scratch: &'s mut AppendVec<StackElement>,
    ) -> Self {
        debug_assert!(scratch.is_empty());
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}

impl ViewElement for StackElement {
    type Mut<'w> = StackElementMut<'w>;
}

impl SuperElement<Self, ViewCtx> for StackElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = StackElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for StackElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self {
            child: child.erased(),
            alignment: None,
        }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::Stack::child_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<StackElement> for StackSplice<'_, '_> {
    fn insert(&mut self, element: StackElement) {
        widgets::Stack::insert_child(
            &mut self.element,
            self.idx,
            element.child.new_widget,
            element.alignment,
        );
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<StackElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            widgets::Stack::insert_child(
                &mut self.element,
                self.idx,
                element.child.new_widget,
                element.alignment,
            );
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, StackElement>) -> R) -> R {
        let child = StackElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, StackElement>) -> R) -> R {
        let ret = {
            let child = StackElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::Stack::remove_child(&mut self.element, self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn index(&self) -> usize {
        self.idx
    }
}

/// `StackSequence` is what allows an input to the grid that contains all the grid elements.
pub trait StackSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, StackElement>
{
}

impl<Seq, State, Action> StackSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, StackElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a grid item
pub trait StackExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`CrossAxisAlignment`] to this view. This allows the view
    /// to be placed as a child within a [`Stack`] [`View`].

    // TODO - Example
    fn stack_item(self, alignment: CrossAxisAlignment) -> StackItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        stack_item(self, Some(alignment))
    }
}

impl<State, Action, V: WidgetView<State, Action>> StackExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Stack`] [`View`]
pub struct StackItem<V, State, Action> {
    view: V,
    alignment: Option<CrossAxisAlignment>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Creates a [`StackItem`] from a view and [`StackParams`].
pub fn stack_item<V, State, Action>(
    view: V,
    alignment: Option<CrossAxisAlignment>,
) -> StackItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    StackItem {
        view,
        alignment,
        phantom: PhantomData,
    }
}

impl<V, State, Action> ViewMarker for StackItem<V, State, Action> {}

impl<State, Action, V> View<State, Action, ViewCtx> for StackItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = StackElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (
            StackElement {
                child: pod.erased(),
                alignment: self.alignment,
            },
            state,
        )
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
                widgets::Stack::update_child_alignment(
                    &mut element.parent,
                    element.idx,
                    self.alignment,
                );
            }
            let mut child = widgets::Stack::child_mut(&mut element.parent, element.idx);
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
        let mut child = widgets::Stack::child_mut(&mut element.parent, element.idx);
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
        let mut child = widgets::Stack::child_mut(&mut element.parent, element.idx);
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}
