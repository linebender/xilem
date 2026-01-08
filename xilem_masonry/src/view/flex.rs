// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::{CollectionWidget, FromDynWidget, Widget, WidgetMut};
use masonry::kurbo::Axis;
use masonry::layout::Length;
pub use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
pub use masonry::widgets::FlexParams;
use masonry::widgets::{self};

use crate::core::{
    AppendVec, Arg, ElementSplice, MessageCtx, MessageResult, Mut, SuperElement, View,
    ViewArgument, ViewElement, ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};
use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};

/// A layout which defines how items will be arranged in rows or columns.
///
/// Most use cases for flexible layouts should use one of [`flex_row`] and [`flex_col`].
///
/// Every child has a `Flex` specific configuration in the form of [`FlexParams`].
/// This configuration sets the flex factor, the basis, and the cross axis alignment.
///
/// The basis determines the starting size of each child. For fixed children this will default to
/// [`FlexBasis::Auto`] which means that they will be at their intrinsic preferred size.
/// Flexible children, that is children with a flex factor greater than zero,
/// will default to [`FlexBasis::Zero`] and thus fully depend on extra space distribution.
///
/// Once all the bases have been resolved, all the remaining free space will get
/// distributed among its children based on everyone's share of the sum of all flex factors.
/// Fixed children have a flex factor of zero, so they don't get anything and stay at their basis.
/// Flexible children will get extra space on top of their basis.
///
/// If there is at least one flexible child, it will use up all the extra space.
/// However, if there are only fixed children and there is extra space,
/// then that gets distributed according to [`MainAxisAlignment`].
///
/// There is currently no fine-grained support for flex grow or flex shrink.
/// Instead every child gets their size decided in one shot, as described above.
///
/// # Example
/// ```rust,no_run
/// # use xilem_masonry as xilem;
/// use xilem::masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
/// use xilem::masonry::kurbo::Axis;
/// use xilem::masonry::layout::AsUnit;
/// use xilem::view::{button, text_button, flex, label, sized_box, FlexExt as _, FlexSpacer, Label};
/// use xilem::style::Style;
/// use xilem::WidgetView;
/// use xilem::core::Edit;
///
/// /// A component to make a bigger than usual button.
/// fn big_button<F: Fn(&mut i32) + Send + Sync + 'static>(
///     label: impl Into<Label>,
///     callback: F,
/// ) -> impl WidgetView<Edit<i32>> {
///     // This being fully specified is "a known limitation of the trait solver"
///     button::<Edit<i32>, _, _, F>(label.into(), callback)
///         .dims(40.px())
/// }
///
/// fn app_logic(data: &mut i32) -> impl WidgetView<Edit<i32>> + use<> {
///     flex(Axis::Horizontal, (
///         FlexSpacer::Fixed(30.px()),
///         big_button("-", |data| {
///             *data -= 1;
///         }),
///         FlexSpacer::Flex(1.0),
///         label(format!("count: {}", data)).text_size(32.).flex(5.0),
///         FlexSpacer::Flex(1.0),
///         big_button("+", |data| {
///             *data += 1;
///         }),
///         FlexSpacer::Fixed(30.px()),
///     ))
///     .main_axis_alignment(MainAxisAlignment::Center)
///     .cross_axis_alignment(CrossAxisAlignment::Center)
/// }
/// ```
///
/// [`FlexBasis::Auto`]: masonry::widgets::FlexBasis::Auto
/// [`FlexBasis::Zero`]: masonry::widgets::FlexBasis::Zero
pub fn flex<State: ViewArgument, Action, Seq: FlexSequence<State, Action>>(
    axis: Axis,
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    Flex {
        axis,
        sequence,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        phantom: PhantomData,
    }
}

/// A layout where the children are laid out in a row.
///
/// This is equivalent to [`flex`] with a pre-applied horizontal
/// [`direction`](Flex::direction).
/// We recommend reading that type's documentation for a detailed
/// explanation of this component's layout model.
pub fn flex_row<State: ViewArgument, Action, Seq: FlexSequence<State, Action>>(
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    flex(Axis::Horizontal, sequence)
}

/// A layout where the children are laid out in a column.
///
/// This is equivalent to [`flex`] with a pre-applied vertical
/// [`direction`](Flex::direction).
/// We recommend reading that type's documentation for a detailed
/// explanation of this component's layout model.
pub fn flex_col<State: ViewArgument, Action, Seq: FlexSequence<State, Action>>(
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    flex(Axis::Vertical, sequence)
}

/// The [`View`] created by [`flex`] from a sequence.
///
/// See `flex` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Flex<Seq, State, Action = ()> {
    sequence: Seq,
    axis: Axis,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> Flex<Seq, State, Action> {
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
}

mod hidden {
    use super::FlexItem;
    use crate::core::{AppendVec, View, ViewArgument};
    use crate::view::FlexElement;
    use crate::{AnyWidgetView, ViewCtx};

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct FlexState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<FlexElement>,
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct AnyFlexChildState<State: ViewArgument, Action: 'static> {
        /// Just the optional view state of the flex item view
        #[allow(
            clippy::type_complexity,
            reason = "There's no way to avoid spelling out this type."
        )]
        pub(crate) inner: Option<
            <FlexItem<Box<AnyWidgetView<State, Action>>, State, Action> as View<
                State,
                Action,
                ViewCtx,
            >>::ViewState,
        >,
        /// The generational id handling is essentially very similar to that of the `Option<impl ViewSequence>`,
        /// where `None` would represent a Spacer, and `Some` a view
        pub(crate) generation: u64,
    }
}

use hidden::{AnyFlexChildState, FlexState};

impl<Seq, State, Action> ViewMarker for Flex<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Flex<Seq, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    Seq: FlexSequence<State, Action>,
{
    type Element = Pod<widgets::Flex>;

    type ViewState = FlexState<Seq::SeqState>;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Flex::for_axis(self.axis)
            .cross_axis_alignment(self.cross_axis_alignment)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = match child {
                FlexElement::Child(child, params) => widget.with(child.new_widget, params),
                FlexElement::FixedSpacer(size) => widget.with_fixed_spacer(size),
                FlexElement::FlexSpacer(flex) => widget.with_spacer(flex),
            }
        }
        let pod = ctx.create_pod(widget);
        let state = FlexState {
            seq_state,
            scratch: elements,
        };

        (pod, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        FlexState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        if prev.axis != self.axis {
            widgets::Flex::set_direction(&mut element, self.axis);
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            widgets::Flex::set_cross_axis_alignment(&mut element, self.cross_axis_alignment);
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            widgets::Flex::set_main_axis_alignment(&mut element, self.main_axis_alignment);
        }
        let mut splice = FlexSplice::new(element, scratch);
        self.sequence
            .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        FlexState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = FlexSplice::new(element, scratch);
        self.sequence.seq_teardown(seq_state, ctx, &mut splice);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        FlexState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let mut splice = FlexSplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

/// A child element of a [`Flex`] view.
pub enum FlexElement {
    /// Child widget.
    Child(Pod<dyn Widget>, FlexParams),
    /// Child spacer with fixed size.
    FixedSpacer(Length),
    /// Child spacer with flex size.
    FlexSpacer(f64),
}

/// A mutable reference to a [`FlexElement`], used internally by Xilem traits.
pub struct FlexElementMut<'w> {
    parent: WidgetMut<'w, widgets::Flex>,
    idx: usize,
}

struct FlexSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::Flex>,
    scratch: &'s mut AppendVec<FlexElement>,
}

impl<'w, 's> FlexSplice<'w, 's> {
    fn new(element: WidgetMut<'w, widgets::Flex>, scratch: &'s mut AppendVec<FlexElement>) -> Self {
        debug_assert!(scratch.is_empty());
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}

impl ViewElement for FlexElement {
    type Mut<'w> = FlexElementMut<'w>;
}

impl SuperElement<Self, ViewCtx> for FlexElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
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

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for FlexElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self::Child(child.erased(), FlexParams::default())
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::Flex::get_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<FlexElement> for FlexSplice<'_, '_> {
    fn insert(&mut self, element: FlexElement) {
        match element {
            FlexElement::Child(child, params) => {
                widgets::Flex::insert(&mut self.element, self.idx, child.new_widget, params);
            }
            FlexElement::FixedSpacer(len) => {
                widgets::Flex::insert_fixed_spacer(&mut self.element, self.idx, len);
            }
            FlexElement::FlexSpacer(len) => {
                widgets::Flex::insert_spacer(&mut self.element, self.idx, len);
            }
        };
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<FlexElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            match element {
                FlexElement::Child(child, params) => {
                    widgets::Flex::insert(&mut self.element, self.idx, child.new_widget, params);
                }
                FlexElement::FixedSpacer(len) => {
                    widgets::Flex::insert_fixed_spacer(&mut self.element, self.idx, len);
                }
                FlexElement::FlexSpacer(len) => {
                    widgets::Flex::insert_spacer(&mut self.element, self.idx, len);
                }
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
        widgets::Flex::remove(&mut self.element, self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn index(&self) -> usize {
        self.idx
    }
}

/// An ordered sequence of views for a [`Flex`] view.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::view::{label, FlexSequence, FlexExt as _};
/// use xilem::core::ViewArgument;
///
/// fn label_sequence<State: ViewArgument>(
///     labels: impl Iterator<Item = &'static str>,
///     flex: f64,
/// ) -> impl FlexSequence<State> {
///     labels.map(|l| label(l).flex(flex)).collect::<Vec<_>>()
/// }
/// ```
pub trait FlexSequence<State: ViewArgument, Action = ()>:
    ViewSequence<State, Action, ViewCtx, FlexElement>
{
}

impl<Seq, State: ViewArgument, Action> FlexSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, FlexElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a flex item, or being able to use it interchangeably with a spacer.
pub trait FlexExt<State: ViewArgument, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<FlexParams>`](`FlexParams`) to this view, can be used as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use xilem::masonry::kurbo::Axis;
    /// use xilem::masonry::layout::AsUnit;
    /// use xilem::view::{text_button, label, flex, CrossAxisAlignment, FlexSpacer, FlexExt};
    /// # use xilem::{WidgetView, core::ViewArgument};
    ///
    /// # fn view<State: ViewArgument>() -> impl WidgetView<State> {
    /// flex(Axis::Vertical, (
    ///     text_button("click me", |_| ()).flex(2.0),
    ///     FlexSpacer::Fixed(2.px()),
    ///     label("a label").flex(CrossAxisAlignment::Fill),
    ///     FlexSpacer::Fixed(2.px()),
    /// ))
    /// # }
    ///
    /// ```
    fn flex(self, params: impl Into<FlexParams>) -> FlexItem<Self, State, Action>
    where
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
    /// # use xilem_masonry as xilem;
    /// use xilem::masonry::kurbo::Axis;
    /// use xilem::masonry::layout::AsUnit;
    /// use xilem::view::{flex, label, FlexSpacer, FlexExt, AnyFlexChild};
    /// # use xilem::{WidgetView, core::ViewArgument};
    ///
    /// # fn view<State: ViewArgument>() -> impl WidgetView<State> {
    /// flex(Axis::Vertical, [label("a label").into_any_flex(), AnyFlexChild::Spacer(FlexSpacer::Fixed(1.px()))])
    /// # }
    ///
    /// ```
    fn into_any_flex(self) -> AnyFlexChild<State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        AnyFlexChild::Item(flex_item(self.boxed(), FlexParams::default()))
    }
}

impl<State: ViewArgument, Action, V: WidgetView<State, Action>> FlexExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Flex`] [`View`].
pub struct FlexItem<V, State, Action> {
    view: V,
    params: FlexParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Applies [`impl Into<FlexParams>`](`FlexParams`) to the [`View`] `V`, can be used as child of a [`Flex`] View.
///
/// # Examples
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::masonry::kurbo::Axis;
/// use xilem::masonry::layout::AsUnit;
/// use xilem::view::{text_button, label, flex_item, flex, CrossAxisAlignment, FlexSpacer};
/// # use xilem::{WidgetView, core::ViewArgument};
///
/// # fn view<State: ViewArgument>() -> impl WidgetView<State> {
/// flex(Axis::Vertical, (
///     flex_item(text_button("click me", |_| ()), 2.0),
///     FlexSpacer::Fixed(2.px()),
///     flex_item(label("a label"), CrossAxisAlignment::Fill),
///     FlexSpacer::Fixed(2.px()),
/// ))
/// # }
///
/// ```
pub fn flex_item<V, State, Action>(
    view: V,
    params: impl Into<FlexParams>,
) -> FlexItem<V, State, Action>
where
    State: ViewArgument,
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
    State: ViewArgument,
    Action: 'static,
    V: WidgetView<State, Action, ViewState: ViewArgument>,
{
    fn from(value: FlexItem<V, State, Action>) -> Self {
        Self::Item(flex_item(value.view.boxed(), value.params))
    }
}

impl<V, State, Action> ViewMarker for FlexItem<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for FlexItem<V, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = FlexElement;

    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (FlexElement::Child(pod.erased(), self.params), state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        {
            if self.params != prev.params {
                widgets::Flex::set_params(&mut element.parent, element.idx, self.params);
            }
            let mut child = widgets::Flex::get_mut(&mut element.parent, element.idx);
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::Flex::get_mut(&mut element.parent, element.idx);
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let mut child = widgets::Flex::get_mut(&mut element.parent, element.idx);
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}

/// A spacer that can be used within a [`Flex`] [`View`]
#[derive(Copy, Clone, PartialEq)]
#[expect(missing_docs, reason = "TODO - Need to document units used.")]
pub enum FlexSpacer {
    Fixed(Length),
    Flex(f64),
}

impl<State, Action> From<FlexSpacer> for AnyFlexChild<State, Action> {
    fn from(spacer: FlexSpacer) -> Self {
        Self::Spacer(spacer)
    }
}

impl ViewMarker for FlexSpacer {}
// This impl doesn't require a view id, as it neither receives, nor sends any messages
// If this should ever change, it's necessary to adjust the `AnyFlexChild` `View` impl
impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for FlexSpacer {
    type Element = FlexElement;

    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let el = match self {
            Self::Fixed(len) => FlexElement::FixedSpacer(*len),
            Self::Flex(flex) => FlexElement::FlexSpacer(*flex),
        };
        (el, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) {
        if self != prev {
            match self {
                Self::Fixed(len) => {
                    widgets::Flex::set_fixed_spacer(&mut element.parent, element.idx, *len);
                }
                Self::Flex(flex) => {
                    widgets::Flex::set_spacer(&mut element.parent, element.idx, *flex);
                }
            };
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) -> MessageResult<Action> {
        unreachable!("FlexSpacer doesn't handle messages but got {message:?}.")
    }
}

/// A widget-type-erased flex child [`View`], can be used within a [`Flex`] [`View`]
pub enum AnyFlexChild<State, Action = ()> {
    /// A child widget.
    Item(FlexItem<Box<AnyWidgetView<State, Action>>, State, Action>),
    /// A spacer.
    Spacer(FlexSpacer),
}

impl FlexSpacer {
    /// Turns this [`FlexSpacer`] into an [`AnyFlexChild`],
    /// which can be used interchangeably with an `FlexItem` (as [`AnyFlexChild`]), as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use xilem::masonry::kurbo::Axis;
    /// use xilem::masonry::layout::AsUnit;
    /// use xilem::view::{flex, FlexSpacer};
    /// # use xilem::{WidgetView, core::ViewArgument};
    ///
    /// # fn view<State: ViewArgument>() -> impl WidgetView<State> {
    /// flex(Axis::Vertical, FlexSpacer::Fixed(2.px()).into_any_flex())
    /// # }
    ///
    /// ```
    pub fn into_any_flex<State, Action>(self) -> AnyFlexChild<State, Action> {
        AnyFlexChild::Spacer(self)
    }
}

impl<State, Action, V> FlexItem<V, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    /// Turns this [`FlexItem`] into an [`AnyFlexChild`]
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use xilem::masonry::kurbo::Axis;
    /// use xilem::view::{flex, flex_item, label};
    /// # use xilem::{WidgetView, core::ViewArgument};
    ///
    /// # fn view<State: ViewArgument>() -> impl WidgetView<State> {
    /// flex(Axis::Vertical, flex_item(label("Industry"), 4.0).into_any_flex())
    /// # }
    ///
    /// ```
    pub fn into_any_flex(self) -> AnyFlexChild<State, Action> {
        AnyFlexChild::Item(flex_item(Box::new(self.view), self.params))
    }
}

impl<State, Action> ViewMarker for AnyFlexChild<State, Action> {}
impl<State, Action> View<State, Action, ViewCtx> for AnyFlexChild<State, Action>
where
    State: ViewArgument,
    Action: 'static,
{
    type Element = FlexElement;

    type ViewState = AnyFlexChildState<State, Action>;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, view_state) = match self {
            Self::Item(flex_item) => {
                let (element, state) = ctx.with_id(ViewId::new(generation), |ctx| {
                    flex_item.build(ctx, app_state)
                });
                (element, Some(state))
            }
            Self::Spacer(spacer) => {
                // We know that the spacer doesn't need any id, as it doesn't receive or sends any messages
                // (Similar to `None` as a ViewSequence)
                let (element, ()) = View::<(), (), ViewCtx>::build(spacer, ctx, ());
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

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        match (prev, self) {
            (Self::Item(prev), Self::Item(this)) => {
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    this.rebuild(
                        prev,
                        view_state.inner.as_mut().unwrap(),
                        ctx,
                        element,
                        app_state,
                    );
                });
            }
            (Self::Spacer(prev), Self::Spacer(this)) => {
                View::<(), (), ViewCtx>::rebuild(this, prev, &mut (), ctx, element, ());
            }
            (Self::Item(prev_flex_item), Self::Spacer(new_spacer)) => {
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
                widgets::Flex::remove(&mut element.parent, element.idx);
                // The Flex item view has just been destroyed, teardown the old view
                // We increment the generation only on the falling edge (new item `FlexSpacer`) by convention
                // This choice has no impact on functionality
                view_state.inner = None;

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                view_state.generation = view_state.generation.wrapping_add(1);
                let (spacer_element, ()) = View::<(), (), ViewCtx>::build(new_spacer, ctx, ());
                match spacer_element {
                    FlexElement::FixedSpacer(len) => {
                        widgets::Flex::insert_fixed_spacer(&mut element.parent, element.idx, len);
                    }
                    FlexElement::FlexSpacer(len) => {
                        widgets::Flex::insert_spacer(&mut element.parent, element.idx, len);
                    }
                    FlexElement::Child(_, _) => unreachable!(),
                };
            }
            (Self::Spacer(prev_spacer), Self::Item(new_flex_item)) => {
                View::<(), (), ViewCtx>::teardown(
                    prev_spacer,
                    &mut (),
                    ctx,
                    FlexElementMut {
                        parent: element.parent.reborrow_mut(),
                        idx: element.idx,
                    },
                );
                widgets::Flex::remove(&mut element.parent, element.idx);

                let (flex_item_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_flex_item.build(ctx, app_state)
                    });
                view_state.inner = Some(child_state);
                if let FlexElement::Child(child, params) = flex_item_element {
                    widgets::Flex::insert(
                        &mut element.parent,
                        element.idx,
                        child.new_widget,
                        params,
                    );
                } else {
                    unreachable!("We just created a new flex item, this should not be reached")
                }
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
            Self::Item(flex_item) => {
                flex_item.teardown(view_state.inner.as_mut().unwrap(), ctx, element);
            }
            Self::Spacer(spacer) => {
                View::<(), (), ViewCtx>::teardown(spacer, &mut (), ctx, element);
            }
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for AnyFlexChild");
        if start.routing_id() != view_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale;
        }
        let Self::Item(flex_item) = self else {
            unreachable!(
                "this should be unreachable as the generation was increased on the falling edge"
            )
        };

        flex_item.message(
            view_state.inner.as_mut().unwrap(),
            message,
            element,
            app_state,
        )
    }
}
