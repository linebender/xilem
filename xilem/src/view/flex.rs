// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    widget::{self, WidgetMut},
    Widget,
};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};

use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};

pub use masonry::widget::{Axis, CrossAxisAlignment, FlexParams, MainAxisAlignment};

pub fn flex<State, Action, Seq: FlexSequence<State, Action>>(
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    Flex {
        sequence,
        axis: Axis::Vertical,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        fill_major_axis: false,
        gap: None,
        phantom: PhantomData,
    }
}

pub struct Flex<Seq, State, Action = ()> {
    sequence: Seq,
    axis: Axis,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    gap: Option<f64>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> Flex<Seq, State, Action> {
    pub fn direction(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn cross_axis_alignment(mut self, axis: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = axis;
        self
    }

    pub fn main_axis_alignment(mut self, axis: MainAxisAlignment) -> Self {
        self.main_axis_alignment = axis;
        self
    }

    pub fn must_fill_major_axis(mut self, fill_major_axis: bool) -> Self {
        self.fill_major_axis = fill_major_axis;
        self
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Equivalent to the css [gap] property.
    /// This gap is also present between spacers.
    ///
    /// Leave unset to use the default spacing, which is [`WIDGET_PADDING_VERTICAL`] for a flex
    /// column and [`WIDGET_PADDING_HORIZONTAL`] for flex row.
    ///
    /// ## Panics
    ///
    /// If `gap` is not a non-negative finite value.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    /// [`WIDGET_PADDING_VERTICAL`]: masonry::theme::WIDGET_PADDING_VERTICAL
    /// [`WIDGET_PADDING_HORIZONTAL`]: masonry::theme::WIDGET_PADDING_VERTICAL
    #[track_caller]
    pub fn gap(mut self, gap: f64) -> Self {
        if gap.is_finite() && gap >= 0.0 {
            self.gap = Some(gap);
        } else {
            // TODO: Don't panic here, for future editor scenarios.
            panic!("Invalid `gap` {gap}, expected a non-negative finite value.")
        }
        self
    }
}

impl<Seq, State, Action> ViewMarker for Flex<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Flex<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FlexSequence<State, Action>,
{
    type Element = Pod<widget::Flex>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::Flex::for_axis(self.axis)
            .raw_gap(self.gap)
            .cross_axis_alignment(self.cross_axis_alignment)
            .must_fill_main_axis(self.fill_major_axis)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = match child {
                FlexElement::Child(child, params) => {
                    widget.with_flex_child_pod(child.inner, params)
                }
                FlexElement::FixedSpacer(size) => widget.with_spacer(size),
                FlexElement::FlexSpacer(flex) => widget.with_flex_spacer(flex),
            }
        }
        (ctx.new_pod(widget), seq_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.axis != self.axis {
            element.set_direction(self.axis);
            ctx.mark_changed();
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            element.set_cross_axis_alignment(self.cross_axis_alignment);
            ctx.mark_changed();
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            element.set_main_axis_alignment(self.main_axis_alignment);
            ctx.mark_changed();
        }
        if prev.fill_major_axis != self.fill_major_axis {
            element.set_must_fill_main_axis(self.fill_major_axis);
            ctx.mark_changed();
        }
        if prev.gap != self.gap {
            element.set_raw_gap(self.gap);
            ctx.mark_changed();
        }
        // TODO: Re-use scratch space?
        let mut splice = FlexSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.is_empty());
        splice.element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = FlexSplice::new(element);
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

#[allow(clippy::large_enum_variant)]
pub enum FlexElement {
    // Avoid making the enum massive for the spacer cases by boxing
    Child(Pod<Box<dyn Widget>>, FlexParams),
    FixedSpacer(f64),
    FlexSpacer(f64),
}

pub struct FlexElementMut<'w> {
    parent: WidgetMut<'w, widget::Flex>,
    idx: usize,
}

struct FlexSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::Flex>,
    scratch: AppendVec<FlexElement>,
}

impl<'w> FlexSplice<'w> {
    fn new(element: WidgetMut<'w, widget::Flex>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ViewElement for FlexElement {
    type Mut<'w> = FlexElementMut<'w>;
}

impl SuperElement<FlexElement, ViewCtx> for FlexElement {
    fn upcast(_ctx: &mut ViewCtx, child: FlexElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, FlexElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = FlexElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for FlexElement {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        FlexElement::Child(ctx.boxed_pod(child), FlexParams::default())
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = this
                .parent
                .child_mut(this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<FlexElement> for FlexSplice<'_> {
    fn insert(&mut self, element: FlexElement) {
        match element {
            FlexElement::Child(child, params) => {
                self.element
                    .insert_flex_child_pod(self.idx, child.inner, params);
            }
            FlexElement::FixedSpacer(len) => self.element.insert_spacer(self.idx, len),
            FlexElement::FlexSpacer(len) => self.element.insert_flex_spacer(self.idx, len),
        };
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<FlexElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            match element {
                FlexElement::Child(child, params) => {
                    self.element
                        .insert_flex_child_pod(self.idx, child.inner, params);
                }
                FlexElement::FixedSpacer(len) => self.element.insert_spacer(self.idx, len),
                FlexElement::FlexSpacer(len) => self.element.insert_flex_spacer(self.idx, len),
            };
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, FlexElement>) -> R) -> R {
        let child = FlexElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, FlexElement>) -> R) -> R {
        let ret = {
            let child = FlexElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        self.element.remove_child(self.idx);
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
/// use xilem::view::{label, FlexSequence, FlexExt as _};
///
/// fn label_sequence<State: 'static>(
///     labels: impl Iterator<Item = &'static str>,
///     flex: f64,
/// ) -> impl FlexSequence<State> {
///     labels.map(|l| label(l).flex(flex)).collect::<Vec<_>>()
/// }
/// ```
pub trait FlexSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, FlexElement>
{
}

impl<Seq, State, Action> FlexSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, FlexElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a flex item, or being able to use it interchangeably with a spacer
pub trait FlexExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<FlexParams>`](`FlexParams`) to this view, can be used as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// use xilem::{view::{button, label, flex, CrossAxisAlignment, FlexSpacer, FlexExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex((
    ///     button("click me", |_| ()).flex(2.0),
    ///     FlexSpacer::Fixed(2.0),
    ///     label("a label").flex(CrossAxisAlignment::Fill),
    ///     FlexSpacer::Fixed(2.0),
    /// ))
    /// # }
    ///
    /// ```
    fn flex(self, params: impl Into<FlexParams>) -> FlexItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        flex_item(self, params)
    }

    /// Turns this [`WidgetView`] into an [`AnyFlexChild`],
    /// which can be used interchangeably with an `FlexSpacer`, as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// use xilem::{view::{flex, label, FlexSpacer, FlexExt, AnyFlexChild}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex([label("a label").into_any_flex(), AnyFlexChild::Spacer(FlexSpacer::Fixed(1.0))])
    /// # }
    ///
    /// ```
    fn into_any_flex(self) -> AnyFlexChild<State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        AnyFlexChild::Item(flex_item(self.boxed(), FlexParams::default()))
    }
}

impl<State, Action, V: WidgetView<State, Action>> FlexExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Flex`] [`View`]
pub struct FlexItem<V, State, Action> {
    view: V,
    params: FlexParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Applies [`impl Into<FlexParams>`](`FlexParams`) to the [`View`] `V`, can be used as child of a [`Flex`] View
///
/// # Examples
/// ```
/// use xilem::view::{button, label, flex_item, flex, CrossAxisAlignment, FlexSpacer};
/// # use xilem::{WidgetView};
///
/// # fn view<State: 'static>() -> impl WidgetView<State> {
/// flex((
///     flex_item(button("click me", |_| ()), 2.0),
///     FlexSpacer::Fixed(2.0),
///     flex_item(label("a label"), CrossAxisAlignment::Fill),
///     FlexSpacer::Fixed(2.0),
/// ))
/// # }
///
/// ```
pub fn flex_item<V, State, Action>(
    view: V,
    params: impl Into<FlexParams>,
) -> FlexItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    FlexItem {
        params: params.into(),
        view,
        phantom: PhantomData,
    }
}

impl<State, Action, V> From<FlexItem<V, State, Action>> for AnyFlexChild<State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action, ViewState: 'static>,
{
    fn from(value: FlexItem<V, State, Action>) -> Self {
        AnyFlexChild::Item(flex_item(value.view.boxed(), value.params))
    }
}

impl<V, State, Action> ViewMarker for FlexItem<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for FlexItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = FlexElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx);
        (FlexElement::Child(ctx.boxed_pod(pod), self.params), state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            if self.params != prev.params {
                element
                    .parent
                    .update_child_flex_params(element.idx, self.params);
            }
            let mut child = element
                .parent
                .child_mut(element.idx)
                .expect("FlexWrapper always has a widget child");
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast());
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = element
            .parent
            .child_mut(element.idx)
            .expect("FlexWrapper always has a widget child");
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}

/// A spacer that can be used within a [`Flex`] [`View`]
#[derive(Copy, Clone, PartialEq)]
pub enum FlexSpacer {
    Fixed(f64),
    Flex(f64),
}

impl<State, Action> From<FlexSpacer> for AnyFlexChild<State, Action> {
    fn from(spacer: FlexSpacer) -> Self {
        AnyFlexChild::Spacer(spacer)
    }
}

impl ViewMarker for FlexSpacer {}
// This impl doesn't require a view id, as it neither receives, nor sends any messages
// If this should ever change, it's necessary to adjust the `AnyFlexChild` `View` impl
impl<State, Action> View<State, Action, ViewCtx> for FlexSpacer {
    type Element = FlexElement;

    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let el = match self {
            FlexSpacer::Fixed(len) => FlexElement::FixedSpacer(*len),
            FlexSpacer::Flex(flex) => FlexElement::FlexSpacer(*flex),
        };
        (el, ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if self != prev {
            match self {
                FlexSpacer::Fixed(len) => element.parent.update_spacer_fixed(element.idx, *len),
                FlexSpacer::Flex(flex) => element.parent.update_spacer_flex(element.idx, *flex),
            };
        }
        element
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[xilem_core::ViewId],
        _: DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        unreachable!()
    }
}

/// A widget-type-erased flex child [`View`], can be used within a [`Flex`] [`View`]
pub enum AnyFlexChild<State, Action = ()> {
    Item(FlexItem<Box<AnyWidgetView<State, Action>>, State, Action>),
    Spacer(FlexSpacer),
}

impl FlexSpacer {
    /// Turns this [`FlexSpacer`] into an [`AnyFlexChild`],
    /// which can be used interchangeably with an `FlexItem` (as [`AnyFlexChild`]), as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// use xilem::{view::{flex, FlexSpacer}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex(FlexSpacer::Fixed(2.0).into_any_flex())
    /// # }
    ///
    /// ```
    pub fn into_any_flex<State, Action>(self) -> AnyFlexChild<State, Action> {
        AnyFlexChild::Spacer(self)
    }
}

impl<State, Action, V> FlexItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    /// Turns this [`FlexItem`] into an [`AnyFlexChild`]
    ///
    /// # Examples
    /// ```
    /// use xilem::view::{flex, flex_item, label};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex(flex_item(label("Industry"), 4.0).into_any_flex())
    /// # }
    ///
    /// ```
    pub fn into_any_flex(self) -> AnyFlexChild<State, Action> {
        AnyFlexChild::Item(flex_item(Box::new(self.view), self.params))
    }
}

#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct AnyFlexChildState<State: 'static, Action: 'static> {
    /// Just the optional view state of the flex item view
    #[allow(clippy::type_complexity)]
    inner: Option<
        <FlexItem<Box<AnyWidgetView<State, Action>>, State, Action> as View<
            State,
            Action,
            ViewCtx,
        >>::ViewState,
    >,
    /// The generational id handling is essentially very similar to that of the `Option<impl ViewSequence>`,
    /// where `None` would represent a Spacer, and `Some` a view
    generation: u64,
}

impl<State, Action> ViewMarker for AnyFlexChild<State, Action> {}
impl<State, Action> View<State, Action, ViewCtx> for AnyFlexChild<State, Action>
where
    State: 'static,
    Action: 'static,
{
    type Element = FlexElement;

    type ViewState = AnyFlexChildState<State, Action>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, view_state) = match self {
            AnyFlexChild::Item(flex_item) => {
                let (element, state) =
                    ctx.with_id(ViewId::new(generation), |ctx| flex_item.build(ctx));
                (element, Some(state))
            }
            AnyFlexChild::Spacer(spacer) => {
                // We know that the spacer doesn't need any id, as it doesn't receive or sends any messages
                // (Similar to `None` as a ViewSequence)
                let (element, ()) = View::<(), (), ViewCtx>::build(spacer, ctx);
                (element, None)
            }
        };
        (
            element,
            AnyFlexChildState {
                inner: view_state,
                generation,
            },
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        match (prev, self) {
            (AnyFlexChild::Item(prev), AnyFlexChild::Item(this)) => ctx
                .with_id(ViewId::new(view_state.generation), |ctx| {
                    this.rebuild(prev, view_state.inner.as_mut().unwrap(), ctx, element)
                }),
            (AnyFlexChild::Spacer(prev), AnyFlexChild::Spacer(this)) => {
                View::<(), (), ViewCtx>::rebuild(this, prev, &mut (), ctx, element)
            }
            (AnyFlexChild::Item(prev_flex_item), AnyFlexChild::Spacer(new_spacer)) => {
                // Run teardown with the old path
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    prev_flex_item.teardown(
                        view_state.inner.as_mut().unwrap(),
                        ctx,
                        FlexElementMut {
                            parent: element.parent.reborrow_mut(),
                            idx: element.idx,
                        },
                    );
                });
                element.parent.remove_child(element.idx);
                // The Flex item view has just been destroyed, teardown the old view
                // We increment the generation only on the falling edge (new item `FlexSpacer`) by convention
                // This choice has no impact on functionality
                view_state.inner = None;

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                view_state.generation = view_state.generation.wrapping_add(1);
                let (spacer_element, ()) = View::<(), (), ViewCtx>::build(new_spacer, ctx);
                match spacer_element {
                    FlexElement::FixedSpacer(len) => element.parent.insert_spacer(element.idx, len),
                    FlexElement::FlexSpacer(len) => {
                        element.parent.insert_flex_spacer(element.idx, len);
                    }
                    FlexElement::Child(_, _) => unreachable!(),
                };
                element
            }
            (AnyFlexChild::Spacer(prev_spacer), AnyFlexChild::Item(new_flex_item)) => {
                View::<(), (), ViewCtx>::teardown(
                    prev_spacer,
                    &mut (),
                    ctx,
                    FlexElementMut {
                        parent: element.parent.reborrow_mut(),
                        idx: element.idx,
                    },
                );
                element.parent.remove_child(element.idx);

                let (flex_item_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_flex_item.build(ctx)
                    });
                view_state.inner = Some(child_state);
                if let FlexElement::Child(child, params) = flex_item_element {
                    element
                        .parent
                        .insert_flex_child_pod(element.idx, child.inner, params);
                } else {
                    unreachable!("We just created a new flex item, this should not be reached")
                }

                element
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        match self {
            AnyFlexChild::Item(flex_item) => {
                flex_item.teardown(view_state.inner.as_mut().unwrap(), ctx, element);
            }
            AnyFlexChild::Spacer(spacer) => {
                View::<(), (), ViewCtx>::teardown(spacer, &mut (), ctx, element);
            }
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for AnyFlexChild");
        if start.routing_id() != view_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        let AnyFlexChild::Item(flex_item) = self else {
            unreachable!(
                "this should be unreachable as the generation was increased on the falling edge"
            )
        };

        flex_item.message(view_state.inner.as_mut().unwrap(), rest, message, app_state)
    }
}
