// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::core::{
    AppendVec, ElementSplice, MessageCtx, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewId, ViewMarker, ViewPathTracker, ViewSequence,
};
use crate::{AnyWidgetView, Pod, ViewCtx, WidgetView};
use masonry::core::{CollectionWidget, FromDynWidget, Widget, WidgetMut};
use masonry::layout::Length;
use masonry::widgets;

pub use widgets::taffy;

/// A layout where the children are laid out in a row using
/// [CSS Flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Flexible_box_layout) algorithm.
pub fn taffy_row<State, Action, Seq: TaffySequence<State, Action>>(
    sequence: Seq,
) -> Taffy<FlexContainerParams, Seq, State, Action> {
    Taffy {
        sequence,
        params: FlexContainerParams::default(),
        phantom: PhantomData,
    }
}

/// A layout where the children are laid out in a column using
/// [CSS Flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Flexible_box_layout) algorithm.
pub fn taffy_col<State, Action, Seq: TaffySequence<State, Action>>(
    sequence: Seq,
) -> Taffy<FlexContainerParams, Seq, State, Action> {
    taffy_row(sequence).direction(taffy::FlexDirection::Column)
}

/// A layout where the children are laid out in a grid using
/// [CSS Grid](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Grid_layout) algorithm.
pub fn taffy_grid<State, Action, Seq: TaffySequence<State, Action>>(
    sequence: Seq,
) -> Taffy<GridContainerParams, Seq, State, Action> {
    Taffy {
        sequence,
        params: GridContainerParams::default(),
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`taffy_row`], [`taffy_col`] or [`taffy_grid`] from a sequence.
pub struct Taffy<Params, Seq, State, Action = ()> {
    sequence: Seq,
    params: Params,

    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Describes how a grid track (row/column) should be sized.
///
/// For more details, see [here](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/grid-template-columns).
#[derive(Default, Clone, Copy, PartialEq)]
pub enum GridTrackSize {
    /// At least the minimum intrinsic size of the content and
    /// at most the maximum intrinsic size of the content.
    #[default]
    Auto,

    /// Minimum intrinsic size of the content.
    MinContent,

    /// Maximum intrinsic size of the content
    MaxContent,

    /// Fit content within the given size.
    FitContent(f32),

    /// Fixed at the given size.
    Fixed(f32),

    /// Fixed at the size that is equal to the size of the grid container
    /// times the given parameter.
    ///
    /// The parameter is in normalized percentage, e.g. 0.3, not 30%.
    Percentage(f32),

    /// Fraction of remaining space.
    Fraction(f32),
}

impl GridTrackSize {
    fn to_taffy_track_sizing_fn(self) -> taffy::TrackSizingFunction {
        use masonry::widgets::taffy::style_helpers::*;
        match self {
            Self::Auto => taffy::TrackSizingFunction::AUTO,
            Self::MinContent => taffy::TrackSizingFunction::MIN_CONTENT,
            Self::MaxContent => taffy::TrackSizingFunction::MAX_CONTENT,
            Self::FitContent(s) => {
                taffy::TrackSizingFunction::fit_content(taffy::LengthPercentage::length(s))
            }
            Self::Fixed(s) => taffy::TrackSizingFunction::from_length(s),
            Self::Percentage(s) => taffy::TrackSizingFunction::from_percent(s),
            Self::Fraction(s) => taffy::TrackSizingFunction::from_fr(s),
        }
    }
    fn to_taffy_template(template: &[Self]) -> Vec<taffy::GridTemplateComponent<String>> {
        template
            .iter()
            .copied()
            .map(Self::to_taffy_track_sizing_fn)
            .map(taffy::GridTemplateComponent::Single)
            .collect()
    }
}

impl<Seq, State, Action> Taffy<FlexContainerParams, Seq, State, Action> {
    /// Which direction does the main axis flow in?
    pub fn direction(mut self, direction: taffy::FlexDirection) -> Self {
        self.params.direction = direction;
        self
    }

    /// Should elements wrap, or stay in a single line?
    pub fn wrap(mut self, wrap: taffy::FlexWrap) -> Self {
        self.params.wrap = wrap;
        self
    }

    /// How this node's children aligned in the cross/block axis?
    pub fn align_items(mut self, align_items: taffy::AlignItems) -> Self {
        self.params.align_items = Some(align_items);
        self
    }

    /// How should content contained within this item be aligned in the cross/block axis
    pub fn align_content(mut self, align_content: taffy::AlignContent) -> Self {
        self.params.align_content = Some(align_content);
        self
    }

    /// How should content contained within this item be aligned in the main/inline axis
    pub fn justify_content(mut self, justify_content: taffy::JustifyContent) -> Self {
        self.params.justify_content = Some(justify_content);
        self
    }
}

impl<Seq, State, Action> Taffy<GridContainerParams, Seq, State, Action> {
    /// Adds a row to the grid.
    pub fn row(mut self, row: GridTrackSize) -> Self {
        self.params.rows.push(row);
        self
    }

    /// Adds a column to the grid.
    pub fn column(mut self, column: GridTrackSize) -> Self {
        self.params.columns.push(column);
        self
    }

    /// Defines the track sizes (heights) of the grid rows
    pub fn rows(mut self, rows: impl IntoIterator<Item = GridTrackSize>) -> Self {
        self.params.rows = rows.into_iter().collect();
        self
    }

    /// Defines the track sizes (widths) of the grid columns
    pub fn columns(mut self, columns: impl IntoIterator<Item = GridTrackSize>) -> Self {
        self.params.columns = columns.into_iter().collect();
        self
    }

    /// Controls how items get placed into the grid for auto-placed items
    pub fn auto_flow(mut self, auto_flow: taffy::GridAutoFlow) -> Self {
        self.params.auto_flow = auto_flow;
        self
    }

    /// How this node's children aligned in the cross/block axis?
    pub fn align_items(mut self, align_items: taffy::AlignItems) -> Self {
        self.params.align_items = Some(align_items);
        self
    }

    /// How this node's children should be aligned in the inline axis
    pub fn justify_items(mut self, justify_items: taffy::AlignItems) -> Self {
        self.params.justify_items = Some(justify_items);
        self
    }

    /// How should content contained within this item be aligned in the cross/block axis
    pub fn align_content(mut self, align_content: taffy::AlignContent) -> Self {
        self.params.align_content = Some(align_content);
        self
    }

    /// How should content contained within this item be aligned in the main/inline axis
    pub fn justify_content(mut self, justify_content: taffy::JustifyContent) -> Self {
        self.params.justify_content = Some(justify_content);
        self
    }
}

mod hidden {
    use super::{GridTrackSize, TaffyElement, TaffyItem, taffy};
    use crate::core::{AppendVec, View};
    use crate::{AnyWidgetView, ViewCtx};

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub trait ContainerParams: Clone + PartialEq {
        type ChildParams: ChildParams;
        fn to_taffy_style(&self) -> taffy::Style;
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub trait ChildParams: Clone + PartialEq {
        fn to_taffy_style(&self) -> taffy::Style;
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Default, Clone, PartialEq)]
    pub struct FlexContainerParams {
        /// Which direction does the main axis flow in?
        pub direction: taffy::FlexDirection,

        /// Should elements wrap, or stay in a single line?
        pub wrap: taffy::FlexWrap,

        /// How this node's children aligned in the cross/block axis?
        pub align_items: Option<taffy::AlignItems>,

        /// How should content contained within this item be aligned in the cross/block axis
        pub align_content: Option<taffy::AlignContent>,

        /// How should content contained within this item be aligned in the main/inline axis
        pub justify_content: Option<taffy::JustifyContent>,
    }
    impl ContainerParams for FlexContainerParams {
        type ChildParams = FlexChildParams;
        fn to_taffy_style(&self) -> taffy::Style {
            taffy::Style {
                flex_direction: self.direction,
                flex_wrap: self.wrap,
                align_items: self.align_items,
                align_content: self.align_content,
                justify_content: self.justify_content,
                ..Default::default()
            }
        }
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Clone, PartialEq)]
    pub struct FlexChildParams {
        /// Sets the initial main axis size of the item
        pub basis: Option<f32>,

        /// The relative rate at which this item grows when it is expanding to fill space
        ///
        /// 0.0 is the default value, and this value must be positive.
        pub grow: f32,

        /// The relative rate at which this item shrinks when it is contracting to fit into space
        ///
        /// 1.0 is the default value, and this value must be positive.
        pub shrink: f32,

        /// How this node should be aligned in the cross/block axis
        /// Falls back to the parents [`AlignItems`] if not set
        pub align_self: Option<taffy::AlignSelf>,
    }
    impl Default for FlexChildParams {
        fn default() -> Self {
            Self {
                basis: None,
                grow: 0.,
                shrink: 1.,
                align_self: None,
            }
        }
    }
    impl ChildParams for FlexChildParams {
        fn to_taffy_style(&self) -> taffy::Style {
            taffy::Style {
                flex_basis: self
                    .basis
                    .map_or_else(taffy::Dimension::auto, taffy::Dimension::length),
                flex_grow: self.grow,
                flex_shrink: self.shrink,
                align_self: self.align_self,
                ..Default::default()
            }
        }
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Default, Clone, PartialEq)]
    pub struct GridContainerParams {
        /// Defines the track sizes (heights) of the grid rows
        pub rows: Vec<GridTrackSize>,

        /// Defines the track sizes (widths) of the grid columns
        pub columns: Vec<GridTrackSize>,

        /// Controls how items get placed into the grid for auto-placed items
        pub auto_flow: taffy::GridAutoFlow,

        /// How this node's children aligned in the cross/block axis?
        pub align_items: Option<taffy::AlignItems>,

        /// How this node's children should be aligned in the inline axis
        pub justify_items: Option<taffy::AlignItems>,

        /// How should content contained within this item be aligned in the cross/block axis
        pub align_content: Option<taffy::AlignContent>,

        /// How should content contained within this item be aligned in the main/inline axis
        pub justify_content: Option<taffy::JustifyContent>,
    }
    impl ContainerParams for GridContainerParams {
        type ChildParams = GridChildParams;
        fn to_taffy_style(&self) -> taffy::Style {
            taffy::Style {
                grid_template_columns: GridTrackSize::to_taffy_template(&self.columns),
                grid_template_rows: GridTrackSize::to_taffy_template(&self.rows),
                grid_auto_flow: self.auto_flow,
                align_items: self.align_items,
                justify_items: self.justify_items,
                align_content: self.align_content,
                justify_content: self.justify_content,
                ..Default::default()
            }
        }
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    #[derive(Default, Clone, PartialEq)]
    pub struct GridChildParams {
        /// Defines which column in the grid the item should start
        pub x: Option<i16>,

        /// Defines which row in the grid the item should start
        pub y: Option<i16>,

        /// Defines how many column in the grid the item should occupy
        pub width: Option<u16>,

        /// Defines how many row in the grid the item should occupy
        pub height: Option<u16>,

        /// How this node should be aligned in the cross/block axis
        /// Falls back to the parents [`AlignItems`] if not set
        pub align_self: Option<taffy::AlignSelf>,

        /// How this node should be aligned in the inline axis
        /// Falls back to the parents [`JustifyItems`] if not set
        pub justify_self: Option<taffy::AlignSelf>,
    }
    impl ChildParams for GridChildParams {
        fn to_taffy_style(&self) -> taffy::Style {
            use taffy::{GridPlacement, Line};
            fn to_placement(start: Option<i16>, span: Option<u16>) -> Line<GridPlacement> {
                Line {
                    start: start.map_or(GridPlacement::Auto, |s| GridPlacement::Line(s.into())),
                    end: span.map_or(GridPlacement::Auto, GridPlacement::Span),
                }
            }
            taffy::Style {
                grid_column: to_placement(self.x, self.width),
                grid_row: to_placement(self.y, self.height),
                align_self: self.align_self,
                justify_self: self.justify_self,
                ..Default::default()
            }
        }
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct TaffyState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<TaffyElement>,
    }

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct AnyTaffyChildState<Params: ChildParams + 'static, State: 'static, Action: 'static> {
        /// Just the optional view state of the flex item view
        #[allow(
            clippy::type_complexity,
            reason = "There's no way to avoid spelling out this type."
        )]
        pub(crate) inner: Option<
            <TaffyItem<Params, Box<AnyWidgetView<State, Action>>, State, Action> as View<
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

use hidden::*;

// unsafe impl<Seq: Sync, State, Action> Sync for Taffy<Seq, State, Action> {}
// unsafe impl<Seq: Send, State, Action> Send for Taffy<Seq, State, Action> {}
impl<Params, Seq, State, Action> ViewMarker for Taffy<Params, Seq, State, Action> where
    Params: ContainerParams
{
}
impl<Params, State, Action, Seq> View<State, Action, ViewCtx> for Taffy<Params, Seq, State, Action>
where
    Params: ContainerParams + 'static,
    State: 'static,
    Action: 'static,
    Seq: TaffySequence<State, Action>,
{
    type Element = Pod<widgets::Taffy>;

    type ViewState = TaffyState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Taffy::new(self.params.to_taffy_style());
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = match child {
                TaffyElement::Child(child, style) => widget.with(child.new_widget, style),
                TaffyElement::Spacer(style) => widget.with_spacer(style),
            }
        }

        let pod = ctx.create_pod(widget);
        let state = TaffyState {
            seq_state,
            scratch: elements,
        };
        (pod, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        TaffyState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.params != self.params {
            widgets::Taffy::set_container_style(&mut element, self.params.to_taffy_style());
        }

        let mut splice = TaffySplice::new(element, scratch);
        self.sequence
            .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);

        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        TaffyState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = TaffySplice::new(element, scratch);
        self.sequence.seq_teardown(seq_state, ctx, &mut splice);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        TaffyState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = TaffySplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

impl ViewElement for TaffyElement {
    type Mut<'w> = TaffyElementMut<'w>;
}

impl SuperElement<Self, ViewCtx> for TaffyElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = TaffyElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for TaffyElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self::Child(Box::new(child.erased()), taffy::Style::DEFAULT)
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::Taffy::get_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// Used for building and rebuilding the ViewSequence
impl ElementSplice<TaffyElement> for TaffySplice<'_, '_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<TaffyElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            match element {
                TaffyElement::Child(child, style) => {
                    widgets::Taffy::insert(&mut self.element, self.idx, child.new_widget, style);
                }
                TaffyElement::Spacer(style) => {
                    widgets::Taffy::insert_spacer(&mut self.element, self.idx, style);
                }
            };
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: TaffyElement) {
        match element {
            TaffyElement::Child(child, style) => {
                widgets::Taffy::insert(&mut self.element, self.idx, child.new_widget, style);
            }
            TaffyElement::Spacer(style) => {
                widgets::Taffy::insert_spacer(&mut self.element, self.idx, style);
            }
        };
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, TaffyElement>) -> R) -> R {
        let child = TaffyElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, TaffyElement>) -> R) -> R {
        let ret = {
            let child = TaffyElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::Taffy::remove(&mut self.element, self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn index(&self) -> usize {
        self.idx
    }
}

/// `TaffySequence` is what allows an input to Taffy that contains all the elements.
pub trait TaffySequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, TaffyElement>
{
}

impl<Seq, State, Action> TaffySequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, TaffyElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a taffy item.
pub trait TaffyExt<State: 'static, Action>: WidgetView<State, Action> {
    /// Applies the flex parameters to this view, can be used as child of a [`Taffy`] flex [`View`].
    fn taffy_flex(
        self,
        params: impl Into<FlexChildParams>,
    ) -> TaffyItem<FlexChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        taffy_item(self, params)
    }

    /// Applies the initial main axis size to this view, can be used as child of a [`Taffy`] flex [`View`].
    fn basis(self, basis: f32) -> TaffyItem<FlexChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_flex(FlexChildParams::default()).basis(basis)
    }

    /// Applies the relative rate of growth to this view, can be used as child of a [`Taffy`] flex [`View`].
    fn grow(self, grow: f32) -> TaffyItem<FlexChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_flex(FlexChildParams::default()).grow(grow)
    }

    /// Applies the relative rate of shrinking to this view, can be used as child of a [`Taffy`] flex [`View`].
    fn shrink(self, shrink: f32) -> TaffyItem<FlexChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_flex(FlexChildParams::default()).shrink(shrink)
    }

    /// Turns this view into an [`AnyTaffyFlexChild`], which can be used interchangeably
    /// with an [`TaffySpacer`] (as [`AnyTaffyFlexChild`]), as child of a [`Taffy`] flex [`View`].
    fn into_any_taffy_flex(self) -> AnyTaffyFlexChild<State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        AnyTaffyFlexChild::Item(self.boxed().taffy_flex(FlexChildParams::default()))
    }

    /// Applies the grid parameters to this view, can be used as child of a [`Taffy`] grid [`View`].
    fn taffy_grid(
        self,
        params: impl Into<GridChildParams>,
    ) -> TaffyItem<GridChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        taffy_item(self, params)
    }

    /// Applies the starting column number to this view, can be used as child of a [`Taffy`] grid [`View`].
    fn col(self, col: i16) -> TaffyItem<GridChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_grid(GridChildParams::default()).col(col)
    }

    /// Applies the starting row numer to this view, can be used as child of a [`Taffy`] grid [`View`].
    fn row(self, row: i16) -> TaffyItem<GridChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_grid(GridChildParams::default()).row(row)
    }

    /// Applies the number of occupied columns to this view, can be used as child of a [`Taffy`] grid [`View`].
    fn col_span(self, col_span: u16) -> TaffyItem<GridChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_grid(GridChildParams::default())
            .col_span(col_span)
    }

    /// Applies the number of occupied rows to this view, can be used as child of a [`Taffy`] grid [`View`].
    fn row_span(self, row_span: u16) -> TaffyItem<GridChildParams, Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        self.taffy_grid(GridChildParams::default())
            .row_span(row_span)
    }
}

impl<State: 'static, Action, V: WidgetView<State, Action>> TaffyExt<State, Action> for V {}

/// A child element of a [`Taffy`] flex view.
pub enum TaffyElement {
    /// Child widget.
    Child(Box<Pod<dyn Widget>>, taffy::Style),
    /// Child spacer.
    Spacer(taffy::Style),
}

/// A mutable reference to a [`TaffyElement`], used internally by Xilem traits.
pub struct TaffyElementMut<'w> {
    parent: WidgetMut<'w, widgets::Taffy>,
    idx: usize,
}

// Used for manipulating the ViewSequence.
struct TaffySplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::Taffy>,
    scratch: &'s mut AppendVec<TaffyElement>,
}

impl<'w, 's> TaffySplice<'w, 's> {
    fn new(
        element: WidgetMut<'w, widgets::Taffy>,
        scratch: &'s mut AppendVec<TaffyElement>,
    ) -> Self {
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}

/// A `WidgetView` that can be used within a [`Taffy`] [`View`].
pub struct TaffyItem<Params, V, State, Action> {
    view: V,
    params: Params,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> TaffyItem<FlexChildParams, V, State, Action> {
    /// Sets the initial main axis size of the item
    pub fn basis(mut self, basis: f32) -> Self {
        self.params.basis = Some(basis);
        self
    }

    /// The relative rate at which this item grows when it is expanding to fill space
    pub fn grow(mut self, grow: f32) -> Self {
        self.params.grow = grow;
        self
    }

    /// The relative rate at which this item shrinks when it is contracting to fit into space
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.params.shrink = shrink;
        self
    }

    /// How this node should be aligned in the cross/block axis
    pub fn align_self(mut self, align_self: taffy::AlignItems) -> Self {
        self.params.align_self = Some(align_self);
        self
    }

    /// Turns this view into an [`AnyTaffyFlexChild`], which can be used interchangeably
    /// with an [`TaffySpacer`] (as [`AnyTaffyFlexChild`]), as child of a [`Taffy`] flex [`View`].
    pub fn into_any_taffy_flex(self) -> AnyTaffyFlexChild<State, Action>
    where
        State: 'static,
        Action: 'static,
        V: WidgetView<State, Action>,
    {
        AnyTaffyFlexChild::Item(taffy_item(self.view.boxed(), self.params))
    }
}

impl<V, State, Action> TaffyItem<GridChildParams, V, State, Action> {
    /// Defines which column in the grid the item should start
    pub fn col(mut self, col: i16) -> Self {
        self.params.x = Some(col);
        self
    }

    /// Defines which row in the grid the item should start
    pub fn row(mut self, row: i16) -> Self {
        self.params.y = Some(row);
        self
    }

    /// Defines how many column in the grid the item should occupy
    pub fn col_span(mut self, col_span: u16) -> Self {
        self.params.width = Some(col_span);
        self
    }

    /// Defines how many row in the grid the item should occupy
    pub fn row_span(mut self, row_span: u16) -> Self {
        self.params.height = Some(row_span);
        self
    }

    /// How this node should be aligned in the cross/block axis
    pub fn align_self(mut self, align_self: taffy::AlignItems) -> Self {
        self.params.align_self = Some(align_self);
        self
    }

    /// How this node should be aligned in the inline axis
    pub fn justify_self(mut self, justify_self: taffy::AlignItems) -> Self {
        self.params.justify_self = Some(justify_self);
        self
    }
}

fn taffy_item<Params, V, State, Action>(
    view: V,
    params: impl Into<Params>,
) -> TaffyItem<Params, V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    TaffyItem {
        view,
        params: params.into(),
        phantom: PhantomData,
    }
}

impl<Params, V, State, Action> ViewMarker for TaffyItem<Params, V, State, Action> where
    Params: ChildParams
{
}
impl<Params, State, Action, V> View<State, Action, ViewCtx> for TaffyItem<Params, V, State, Action>
where
    Params: ChildParams + 'static,
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = TaffyElement;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (
            TaffyElement::Child(Box::new(pod.erased()), self.params.to_taffy_style()),
            state,
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
        app_state: &mut State,
    ) {
        if self.params != prev.params {
            widgets::Taffy::set_params(
                &mut element.parent,
                element.idx,
                self.params.to_taffy_style(),
            );
        }
        let mut child = widgets::Taffy::get_mut(&mut element.parent, element.idx);
        self.view
            .rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::Taffy::get_mut(&mut element.parent, element.idx);
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::Taffy::get_mut(&mut element.parent, element.idx);
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}

/// A spacer that can be used within a [`Taffy`] flex [`View`]
#[derive(Clone, PartialEq)]
pub struct TaffySpacer(FlexChildParams);

impl TaffySpacer {
    /// Makes a spacer of fixed size.
    pub fn fixed(length: Length) -> Self {
        Self(FlexChildParams {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "length is expected to be within f32 precision"
            )]
            basis: Some(length.get() as _),
            grow: 0.,
            shrink: 0.,
            align_self: None,
        })
    }
    /// Makes a flexible spacer.
    pub fn flex(flex: f32) -> Self {
        Self(FlexChildParams {
            basis: None,
            grow: flex,
            shrink: 0.,
            align_self: None,
        })
    }

    /// Sets the initial main axis size of the item
    pub fn basis(mut self, basis: f32) -> Self {
        self.0.basis = Some(basis);
        self
    }

    /// The relative rate at which this item grows when it is expanding to fill space
    pub fn grow(mut self, grow: f32) -> Self {
        self.0.grow = grow;
        self
    }

    /// The relative rate at which this item shrinks when it is contracting to fit into space
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.0.shrink = shrink;
        self
    }

    /// Turns this [`TaffySpacer`] into an [`AnyTaffyFlexChild`],
    /// which can be used interchangeably with an `TaffyItem` (as [`AnyTaffyFlexChild`]),
    /// as child of a [`Taffy`] flex [`View`].
    pub fn into_any_taffy_flex<State, Action>(self) -> AnyTaffyFlexChild<State, Action> {
        AnyTaffyFlexChild::Spacer(self)
    }
}

impl<State, Action> From<TaffySpacer> for AnyTaffyFlexChild<State, Action> {
    fn from(spacer: TaffySpacer) -> Self {
        Self::Spacer(spacer)
    }
}

impl ViewMarker for TaffySpacer {}
// This impl doesn't require a view id, as it neither receives, nor sends any messages
// If this should ever change, it's necessary to adjust the `AnyTaffyChild` `View` impl
impl<State: 'static, Action> View<State, Action, ViewCtx> for TaffySpacer {
    type Element = TaffyElement;

    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let el = TaffyElement::Spacer(self.0.to_taffy_style());
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
            widgets::Taffy::set_spacer(&mut element.parent, element.idx, self.0.to_taffy_style());
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<Action> {
        unreachable!("TaffySpacer doesn't handle messages but got {message:?}.")
    }
}

/// A widget-type-erased Taffy child [`View`], can be used within a [`Taffy`] [`View`]
pub enum AnyTaffyFlexChild<State, Action = ()> {
    /// A child widget.
    Item(TaffyItem<FlexChildParams, Box<AnyWidgetView<State, Action>>, State, Action>),
    /// A spacer.
    Spacer(TaffySpacer),
}

impl<State, Action> ViewMarker for AnyTaffyFlexChild<State, Action> {}
impl<State, Action> View<State, Action, ViewCtx> for AnyTaffyFlexChild<State, Action>
where
    State: 'static,
    Action: 'static,
{
    type Element = TaffyElement;

    type ViewState = AnyTaffyChildState<FlexChildParams, State, Action>;

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
            AnyTaffyChildState {
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
                        TaffyElementMut {
                            parent: element.parent.reborrow_mut(),
                            idx: element.idx,
                        },
                    );
                });
                widgets::Taffy::remove(&mut element.parent, element.idx);
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
                    TaffyElement::Spacer(params) => {
                        widgets::Taffy::insert_spacer(&mut element.parent, element.idx, params);
                    }
                    TaffyElement::Child(_, _) => unreachable!(),
                };
            }
            (Self::Spacer(prev_spacer), Self::Item(new_flex_item)) => {
                View::<(), (), ViewCtx>::teardown(
                    prev_spacer,
                    &mut (),
                    ctx,
                    TaffyElementMut {
                        parent: element.parent.reborrow_mut(),
                        idx: element.idx,
                    },
                );
                widgets::Taffy::remove(&mut element.parent, element.idx);

                let (flex_item_element, child_state) = ctx
                    .with_id(ViewId::new(view_state.generation), |ctx| {
                        new_flex_item.build(ctx, app_state)
                    });
                view_state.inner = Some(child_state);
                if let TaffyElement::Child(child, params) = flex_item_element {
                    widgets::Taffy::insert(
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
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let start = message
            .take_first()
            .expect("Id path has elements for AnyTaffyFlexChild");
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
