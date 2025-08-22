// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::style::Style;

use masonry::core::{Axis, FromDynWidget, Widget, WidgetMut};
use masonry::properties::types::Length;
pub use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
pub use masonry::widgets::FlexParams;
use masonry::widgets::{self};

use crate::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};
use crate::{AnyWidgetView, Pod, PropertyTuple as _, ViewCtx, WidgetView};

/// A layout which defines how items will be arranged in rows or columns.
///
/// # Example
/// ```rust,no_run
/// use xilem::masonry::properties::types::{AsUnit, CrossAxisAlignment, MainAxisAlignment};
/// use winit::error::EventLoopError;
/// use xilem::view::{button, flex, label, sized_box, Axis, FlexExt as _, FlexSpacer, Label};
/// use xilem::{EventLoop, WindowOptions, WidgetView, Xilem};
///
/// // A component to make a bigger than usual button
/// fn big_button(
///     label: impl Into<Label>,
///     callback: impl Fn(&mut i32) + Send + Sync + 'static,
/// ) -> impl WidgetView<i32> {
///     sized_box(button(label, callback)).width(40.px()).height(40.px())
/// }
///
/// fn app_logic(data: &mut i32) -> impl WidgetView<i32> + use<> {
///     flex((
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
///     .direction(Axis::Horizontal)
///     .cross_axis_alignment(CrossAxisAlignment::Center)
///     .main_axis_alignment(MainAxisAlignment::Center)
/// }
///
/// fn main() -> Result<(), EventLoopError> {
///     let app = Xilem::new_simple(0, app_logic, WindowOptions::new("Centered Flex"));
///     app.run_in(EventLoop::builder())?;
///     Ok(())
/// }
/// ```
pub fn flex<State, Action, Seq: FlexSequence<State, Action>>(
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    Flex {
        sequence,
        axis: Axis::Vertical,
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        fill_major_axis: false,
        gap: masonry::theme::DEFAULT_GAP,
        properties: FlexProps::default(),
        phantom: PhantomData,
    }
}

/// A layout where the children are laid out in a row.
///
/// This is equivalent to [`flex`] with a pre-applied horizontal
/// [`direction`](Flex::direction).
pub fn flex_row<State, Action, Seq: FlexSequence<State, Action>>(
    sequence: Seq,
) -> Flex<Seq, State, Action> {
    flex(sequence).direction(Axis::Horizontal)
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
    fill_major_axis: bool,
    gap: Length,
    properties: FlexProps,
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
    /// Set whether the container must expand to fill the available space on
    /// its main axis.
    pub fn must_fill_major_axis(mut self, fill_major_axis: bool) -> Self {
        self.fill_major_axis = fill_major_axis;
        self
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Equivalent to the css [gap] property.
    ///
    /// This gap is between any two children, including spacers.
    /// As such, when adding a spacer, you add both the spacer's size (or computed flex size)
    /// and the gap between the spacer and its neighbors.
    /// As such, if you're adding lots of spacers to a flex parent, you may want to set
    /// its gap to zero to make the layout more predictable.
    ///
    /// Leave unset to use the default spacing which is [`DEFAULT_GAP`].
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    /// [`DEFAULT_GAP`]: masonry::theme::DEFAULT_GAP
    #[track_caller]
    pub fn gap(mut self, gap: Length) -> Self {
        self.gap = gap;
        self
    }
}

impl<Seq, S, A> Style for Flex<Seq, S, A> {
    type Props = FlexProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    pub FlexProps;
    Flex<Seq, S, A>;

    Background, 0;
    BorderColor, 1;
    BorderWidth, 2;
    CornerRadius, 3;
    Padding, 4;
);

mod hidden {
    use super::FlexItem;
    use crate::core::{AppendVec, View};
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
    pub struct AnyFlexChildState<State: 'static, Action: 'static> {
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

use hidden::AnyFlexChildState;
use hidden::FlexState;

impl<Seq, State, Action> ViewMarker for Flex<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Flex<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FlexSequence<State, Action>,
{
    type Element = Pod<widgets::Flex>;

    type ViewState = FlexState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Flex::for_axis(self.axis)
            .with_gap(self.gap)
            .cross_axis_alignment(self.cross_axis_alignment)
            .must_fill_main_axis(self.fill_major_axis)
            .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = match child {
                FlexElement::Child(child, params) => {
                    widget.with_flex_child(child.new_widget, params)
                }
                FlexElement::FixedSpacer(size) => widget.with_spacer(size),
                FlexElement::FlexSpacer(flex) => widget.with_flex_spacer(flex),
            }
        }
        let mut pod = ctx.create_pod(widget);
        pod.new_widget.properties = self.properties.build_properties();
        (
            pod,
            FlexState {
                seq_state,
                scratch: elements,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        FlexState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        if prev.axis != self.axis {
            widgets::Flex::set_direction(&mut element, self.axis);
        }
        if prev.cross_axis_alignment != self.cross_axis_alignment {
            widgets::Flex::set_cross_axis_alignment(&mut element, self.cross_axis_alignment);
        }
        if prev.main_axis_alignment != self.main_axis_alignment {
            widgets::Flex::set_main_axis_alignment(&mut element, self.main_axis_alignment);
        }
        if prev.fill_major_axis != self.fill_major_axis {
            widgets::Flex::set_must_fill_main_axis(&mut element, self.fill_major_axis);
        }
        if prev.gap != self.gap {
            widgets::Flex::set_gap(&mut element, self.gap);
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
        app_state: &mut State,
    ) {
        let mut splice = FlexSplice::new(element, scratch);
        self.sequence
            .seq_teardown(seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        FlexState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
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
            let mut child = widgets::Flex::child_mut(&mut this.parent, this.idx)
                .expect("This is supposed to be a widget");
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
                widgets::Flex::insert_flex_child(
                    &mut self.element,
                    self.idx,
                    child.new_widget,
                    params,
                );
            }
            FlexElement::FixedSpacer(len) => {
                widgets::Flex::insert_spacer(&mut self.element, self.idx, len);
            }
            FlexElement::FlexSpacer(len) => {
                widgets::Flex::insert_flex_spacer(&mut self.element, self.idx, len);
            }
        };
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<FlexElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            match element {
                FlexElement::Child(child, params) => {
                    widgets::Flex::insert_flex_child(
                        &mut self.element,
                        self.idx,
                        child.new_widget,
                        params,
                    );
                }
                FlexElement::FixedSpacer(len) => {
                    widgets::Flex::insert_spacer(&mut self.element, self.idx, len);
                }
                FlexElement::FlexSpacer(len) => {
                    widgets::Flex::insert_flex_spacer(&mut self.element, self.idx, len);
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
        widgets::Flex::remove_child(&mut self.element, self.idx);
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

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a flex item, or being able to use it interchangeably with a spacer.
pub trait FlexExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<FlexParams>`](`FlexParams`) to this view, can be used as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// use xilem::masonry::properties::types::AsUnit;
    /// use xilem::{view::{button, label, flex, CrossAxisAlignment, FlexSpacer, FlexExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex((
    ///     button("click me", |_| ()).flex(2.0),
    ///     FlexSpacer::Fixed(2.px()),
    ///     label("a label").flex(CrossAxisAlignment::Fill),
    ///     FlexSpacer::Fixed(2.px()),
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
    /// use xilem::masonry::properties::types::AsUnit;
    /// use xilem::{view::{flex, label, FlexSpacer, FlexExt, AnyFlexChild}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex([label("a label").into_any_flex(), AnyFlexChild::Spacer(FlexSpacer::Fixed(1.px()))])
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
/// use xilem::masonry::properties::types::AsUnit;
/// use xilem::view::{button, label, flex_item, flex, CrossAxisAlignment, FlexSpacer};
/// # use xilem::{WidgetView};
///
/// # fn view<State: 'static>() -> impl WidgetView<State> {
/// flex((
///     flex_item(button("click me", |_| ()), 2.0),
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
        Self::Item(flex_item(value.view.boxed(), value.params))
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

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (FlexElement::Child(pod.erased(), self.params), state)
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
            if self.params != prev.params {
                widgets::Flex::update_child_flex_params(
                    &mut element.parent,
                    element.idx,
                    self.params,
                );
            }
            let mut child = widgets::Flex::child_mut(&mut element.parent, element.idx)
                .expect("FlexWrapper always has a widget child");
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
        let mut child = widgets::Flex::child_mut(&mut element.parent, element.idx)
            .expect("FlexWrapper always has a widget child");
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
        let mut child = widgets::Flex::child_mut(&mut element.parent, element.idx)
            .expect("FlexWrapper always has a widget child");
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
impl<State, Action> View<State, Action, ViewCtx> for FlexSpacer {
    type Element = FlexElement;

    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
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
        _: &mut State,
    ) {
        if self != prev {
            match self {
                Self::Fixed(len) => {
                    widgets::Flex::update_spacer_fixed(&mut element.parent, element.idx, *len);
                }
                Self::Flex(flex) => {
                    widgets::Flex::update_spacer_flex(&mut element.parent, element.idx, *flex);
                }
            };
        }
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        _: &mut State,
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
    /// use xilem::masonry::properties::types::AsUnit;
    /// use xilem::{view::{flex, FlexSpacer}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex(FlexSpacer::Fixed(2.px()).into_any_flex())
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

impl<State, Action> ViewMarker for AnyFlexChild<State, Action> {}
impl<State, Action> View<State, Action, ViewCtx> for AnyFlexChild<State, Action>
where
    State: 'static,
    Action: 'static,
{
    type Element = FlexElement;

    type ViewState = AnyFlexChildState<State, Action>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
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
                let (element, ()) = View::<(), (), ViewCtx>::build(spacer, ctx, &mut ());
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
        app_state: &mut State,
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
                View::<(), (), ViewCtx>::rebuild(this, prev, &mut (), ctx, element, &mut ());
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
                        app_state,
                    );
                });
                widgets::Flex::remove_child(&mut element.parent, element.idx);
                // The Flex item view has just been destroyed, teardown the old view
                // We increment the generation only on the falling edge (new item `FlexSpacer`) by convention
                // This choice has no impact on functionality
                view_state.inner = None;

                // Overflow handling: u64 starts at 0, incremented by 1 always.
                // Can never realistically overflow, scale is too large.
                // If would overflow, wrap to zero. Would need async message sent
                // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
                view_state.generation = view_state.generation.wrapping_add(1);
                let (spacer_element, ()) = View::<(), (), ViewCtx>::build(new_spacer, ctx, &mut ());
                match spacer_element {
                    FlexElement::FixedSpacer(len) => {
                        widgets::Flex::insert_spacer(&mut element.parent, element.idx, len);
                    }
                    FlexElement::FlexSpacer(len) => {
                        widgets::Flex::insert_flex_spacer(&mut element.parent, element.idx, len);
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
                    &mut (),
                );
                widgets::Flex::remove_child(&mut element.parent, element.idx);

                let (flex_item_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_flex_item.build(ctx, app_state)
                    });
                view_state.inner = Some(child_state);
                if let FlexElement::Child(child, params) = flex_item_element {
                    widgets::Flex::insert_flex_child(
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
        app_state: &mut State,
    ) {
        match self {
            Self::Item(flex_item) => {
                flex_item.teardown(view_state.inner.as_mut().unwrap(), ctx, element, app_state);
            }
            Self::Spacer(spacer) => {
                View::<(), (), ViewCtx>::teardown(spacer, &mut (), ctx, element, &mut ());
            }
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
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
