// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::style::Style;

use masonry::core::{FromDynWidget, Widget, WidgetMut};
use masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use masonry::widgets::Axis;
use masonry::widgets::{self};

use crate::core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewSequence,
};
use crate::{Pod, PropertyTuple as _, ViewCtx};

/// A simple parent which lays out children in a non-flex row.
pub fn h_list<State, Action, Seq: ListSequence<State, Action>>(
    sequence: Seq,
) -> List<Seq, State, Action> {
    List {
        sequence,
        axis: Axis::Horizontal,
        gap: masonry::theme::WIDGET_PADDING,
        properties: Default::default(),
        phantom: PhantomData,
    }
}

/// A simple parent which lays out children in a non-flex column.
pub fn v_list<State, Action, Seq: ListSequence<State, Action>>(
    sequence: Seq,
) -> List<Seq, State, Action> {
    List {
        axis: Axis::Vertical,
        ..h_list(sequence)
    }
}

/// The [`View`] created by [`flex`] from a sequence.
///
/// See `flex` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct List<Seq, State, Action = ()> {
    sequence: Seq,
    axis: Axis,
    gap: f64,
    properties: ListProps,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> List<Seq, State, Action> {
    /// Set the spacing along the major axis between any two elements in logical pixels.
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

impl<Seq, S, A> Style for List<Seq, S, A> {
    type Props = ListProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    ListProps;
    List<Seq, S, A>;

    Background, 0;
    BorderColor, 1;
    BorderWidth, 2;
    CornerRadius, 3;
    Padding, 4;
);

impl<Seq, State, Action> ViewMarker for List<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for List<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: ListSequence<State, Action>,
{
    type Element = Pod<widgets::List>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::List::for_axis(self.axis).gap(self.gap);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.into_inner() {
            widget = widget.with_child_pod(child.0.erased_widget_pod());
        }
        let mut pod = ctx.create_pod(widget);
        pod.properties = self.properties.build_properties();
        (pod, seq_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        if prev.axis != self.axis {
            widgets::List::set_direction(&mut element, self.axis);
        }
        if prev.gap != self.gap {
            widgets::List::set_gap(&mut element, self.gap);
        }
        // TODO: Re-use scratch space?
        let mut splice = ListSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice, app_state);
        debug_assert!(splice.scratch.is_empty());
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut splice = ListSplice::new(element);
        self.sequence
            .seq_teardown(view_state, ctx, &mut splice, app_state);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

/// A child element of a [`List`] view.
pub struct ListElement(Pod<dyn Widget>);

/// A mutable reference to a [`ListElement`], used internally by Xilem traits.
pub struct ListElementMut<'w> {
    parent: WidgetMut<'w, widgets::List>,
    idx: usize,
}

struct ListSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widgets::List>,
    scratch: AppendVec<ListElement>,
}

impl<'w> ListSplice<'w> {
    fn new(element: WidgetMut<'w, widgets::List>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ViewElement for ListElement {
    type Mut<'w> = ListElementMut<'w>;
}

impl SuperElement<Self, ViewCtx> for ListElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = ListElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for ListElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self(child.erased())
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::List::child_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<ListElement> for ListSplice<'_> {
    fn insert(&mut self, element: ListElement) {
        widgets::List::insert_child_pod(&mut self.element, self.idx, element.0.erased_widget_pod());
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<ListElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            widgets::List::insert_child_pod(
                &mut self.element,
                self.idx,
                element.0.erased_widget_pod(),
            );
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, ListElement>) -> R) -> R {
        let child = ListElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, ListElement>) -> R) -> R {
        let ret = {
            let child = ListElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::List::remove_child(&mut self.element, self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }
}

/// An ordered sequence of views for a [`Flex`] view.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// use xilem::view::{label, ListSequence, FlexExt as _};
///
/// fn label_sequence<State: 'static>(
///     labels: impl Iterator<Item = &'static str>,
///     flex: f64,
/// ) -> impl ListSequence<State> {
///     labels.map(|l| label(l).flex(flex)).collect::<Vec<_>>()
/// }
/// ```
pub trait ListSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, ListElement>
{
}

impl<Seq, State, Action> ListSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, ListElement>
{
}
