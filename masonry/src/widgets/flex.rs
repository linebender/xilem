// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in a one-dimensional array.

use std::any::TypeId;

use accesskit::{Node, Role};
use masonry_core::core::HasProperty;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Line, Point, Size, Stroke};

use crate::core::{
    AccessCtx, Axis, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::types::Length;
use crate::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::theme::DEFAULT_GAP;
use crate::util::{debug_panic, fill, include_screenshot, stroke};

/// A container with either horizontal or vertical layout.
///
/// This widget is the foundation of most layouts, and is highly configurable.
///
/// The flex model used by Masonry has different behaviour than you might be familiar with from the web.
/// Only children which have an explicit flex factor, set by the first parameter of
/// [`FlexParams::new`](FlexParams::new) being `Some`, will share remaining space flexibly.
/// Children which do not have an explicit flex factor set will be laid out as their natural size.
/// For some widgets (such as [`TextInput`](crate::widgets::TextInput)), this will be
/// all the space made available to the flex (in at least one axis).
/// In the web model, this is equivalent to the default `flex` being `none` (on the web, this is instead `auto`).
/// This can lead to surprising results, including later siblings of the expanded child being pushed off-screen.
/// A general rule of thumb is to set a flex factor on all "large" children in the flex axis, especially
/// portals, sized boxes, and text inputs (in horizontal flex areas).
/// That is, any item which needs to shrink to fit within the viewport should have a
/// flex factor set.
///
/// There is also no support for flex grow or flex shrink; instead, each flexible child takes up
/// the proportion of remaining space (after all "non-flex" children are laid out) specified
/// by its flex factor.
/// In the web flex algorithm, if a widget cannot expand to its target flex size, that remaining space is distributed
/// to the other sibling flex widgets recursively.
/// However, this widget does not implement this behaviour at the moment, as it uses a single-pass layout algorithm.
/// Instead, if a flex child of this widget does not expand to the target size provided by this parent, the difference is distributed
/// to the space between widgets according to this widget's [`MainAxisAlignment`](Flex::set_main_axis_alignment).
///
#[doc = include_screenshot!("flex_col_main_axis_spaceAround.png", "Flex column with multiple labels.")]
pub struct Flex {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    children: Vec<Child>,
    gap: Length,
}

/// Optional parameters for an item in a [`Flex`] container (row or column).
///
/// Generally, when you would like to add a flexible child to a container,
/// you can simply call [`with_flex_child`](Flex::with_flex_child) or [`add_flex_child`](Flex::add_flex_child),
/// passing the child and the desired flex factor as a `f64`, which has an impl of
/// `Into<FlexParams>`.
///
/// You can also add spacers and flexible spacers using e.g. [`with_spacer`](Flex::with_spacer).
/// Spacers are children which take up space but don't paint anything.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct FlexParams {
    flex: Option<f64>,
    alignment: Option<CrossAxisAlignment>,
}

enum Child {
    Fixed {
        widget: WidgetPod<dyn Widget>,
        alignment: Option<CrossAxisAlignment>,
    },
    Flex {
        widget: WidgetPod<dyn Widget>,
        alignment: Option<CrossAxisAlignment>,
        flex: f64,
    },
    FixedSpacer(Length, f64),
    FlexedSpacer(f64, f64),
}

// --- MARK: IMPL FLEX
impl Flex {
    /// Create a new `Flex` oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Self {
            direction: axis,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,
            gap: DEFAULT_GAP,
        }
    }

    /// Create a new horizontal stack.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    ///
    pub fn row() -> Self {
        Self::for_axis(Axis::Horizontal)
    }

    /// Create a new vertical stack.
    ///
    /// The child widgets are laid out vertically, from top to bottom.
    pub fn column() -> Self {
        Self::for_axis(Axis::Vertical)
    }

    /// Builder-style method for specifying the children's [`CrossAxisAlignment`].
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    /// Builder-style method for specifying the children's [`MainAxisAlignment`].
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    /// Builder-style method for setting whether the container must expand
    /// to fill the available space on its main axis.
    pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
        self.fill_major_axis = fill;
        self
    }

    /// Builder-style method for setting a gap along the
    /// major axis between any two elements in logical pixels.
    ///
    /// By default this is [`DEFAULT_GAP`].
    ///
    /// Equivalent to the css [gap] property.
    ///
    /// This gap is between any two children, including spacers.
    /// As such, when adding a spacer, you add both the spacer's size (or computed flex size)
    /// and the gap between the spacer and its neighbors.
    /// As such, if you're adding lots of spacers to a flex parent, you may want to set
    /// its gap to zero to make the layout more predictable.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    // TODO: Semantics - should this include fixed spacers?
    pub fn with_gap(mut self, gap: Length) -> Self {
        self.gap = gap;
        self
    }

    /// Builder-style variant of [`Flex::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(mut self, child: NewWidget<impl Widget + ?Sized>) -> Self {
        let child = Child::Fixed {
            widget: child.erased().to_pod(),
            alignment: None,
        };
        self.children.push(child);
        self
    }

    /// Builder-style method to add a flexible child to the container.
    pub fn with_flex_child(
        mut self,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) -> Self {
        let child = child.erased().to_pod();
        let child = new_flex_child(params.into(), child);
        self.children.push(child);
        self
    }

    /// Builder-style method for adding a fixed-size spacer child to the container.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn with_spacer(mut self, len: Length) -> Self {
        let new_child = Child::FixedSpacer(len, 0.0);
        self.children.push(new_child);
        self
    }

    /// Builder-style method for adding a `flex` spacer child to the container.
    pub fn with_flex_spacer(mut self, flex: f64) -> Self {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        self.children.push(new_child);
        self
    }

    /// Returns the number of children (widgets and spacers) this flex container has.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this flex container has no children (widgets or spacers).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// --- MARK: WIDGETMUT
impl Flex {
    /// Set the flex direction (see [`Axis`]).
    pub fn set_direction(this: &mut WidgetMut<'_, Self>, direction: Axis) {
        this.widget.direction = direction;
        this.ctx.request_layout();
    }

    /// Set the children's [`CrossAxisAlignment`].
    pub fn set_cross_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: CrossAxisAlignment) {
        this.widget.cross_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Set the children's [`MainAxisAlignment`].
    pub fn set_main_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: MainAxisAlignment) {
        this.widget.main_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Set whether the container must expand to fill the available space on
    /// its main axis.
    pub fn set_must_fill_main_axis(this: &mut WidgetMut<'_, Self>, fill: bool) {
        this.widget.fill_major_axis = fill;
        this.ctx.request_layout();
    }

    /// Set the spacing along the major axis between any two elements in logical pixels.
    ///
    /// Equivalent to the css [gap] property.
    ///
    /// This gap is between any two children, including spacers.
    /// As such, using a non-zero gap and also adding spacers may lead to counter-intuitive results.
    /// You should usually pick one or the other.
    ///
    /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    pub fn set_gap(this: &mut WidgetMut<'_, Self>, gap: Length) {
        this.widget.gap = gap;
        this.ctx.request_layout();
    }

    /// Add a non-flex child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Flex::with_child
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let child = Child::Fixed {
            widget: child.erased().to_pod(),
            alignment: None,
        };
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Add a flexible child widget.
    pub fn add_flex_child(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) {
        let child = child.erased().to_pod();
        let child = new_flex_child(params.into(), child);

        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Add an empty spacer child with the given size.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn add_spacer(this: &mut WidgetMut<'_, Self>, len: Length) {
        let new_child = Child::FixedSpacer(len, 0.0);
        this.widget.children.push(new_child);
        this.ctx.request_layout();
    }

    /// Add an empty spacer child with a specific `flex` factor.
    pub fn add_flex_spacer(this: &mut WidgetMut<'_, Self>, flex: f64) {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        this.widget.children.push(new_child);
        this.ctx.request_layout();
    }

    /// Insert a non-flex child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
    ) {
        let child = Child::Fixed {
            widget: child.erased().to_pod(),
            alignment: None,
        };
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Insert a flex child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_flex_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) {
        let child = child.erased().to_pod();
        let child = new_flex_child(params.into(), child);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Insert an empty spacer child with the given size at the given index.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn insert_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, len: Length) {
        let new_child = Child::FixedSpacer(len, 0.0);
        this.widget.children.insert(idx, new_child);
        this.ctx.request_layout();
    }

    /// Add an empty spacer child with a specific `flex` factor.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_flex_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, flex: f64) {
        let flex = if flex >= 0.0 {
            flex
        } else {
            debug_panic!("add_spacer called with negative length: {}", flex);
            0.0
        };
        let new_child = Child::FlexedSpacer(flex, 0.0);
        this.widget.children.insert(idx, new_child);
        this.ctx.request_layout();
    }

    /// Remove the child at `idx`.
    ///
    /// This child can be a widget or a spacer.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        if let Child::Fixed { widget, .. } | Child::Flex { widget, .. } = child {
            this.ctx.remove_child(widget);
        }
        this.ctx.request_layout();
    }

    /// Returns a mutable reference to the child widget at `idx`.
    ///
    /// Returns `None` if the child at `idx` is a spacer.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> Option<WidgetMut<'t, dyn Widget>> {
        let child = match &mut this.widget.children[idx] {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => widget,
            Child::FixedSpacer(..) => return None,
            Child::FlexedSpacer(..) => return None,
        };

        Some(this.ctx.get_mut(child))
    }

    /// Updates the flex parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_flex_params(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        params: impl Into<FlexParams>,
    ) {
        let child = &mut this.widget.children[idx];
        let child_val = std::mem::replace(child, Child::FixedSpacer(Length::ZERO, 0.0));
        let widget = match child_val {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => widget,
            _ => {
                panic!("Can't update flex parameters of a spacer element");
            }
        };
        let new_child = new_flex_child(params.into(), widget);
        *child = new_child;
        this.ctx.children_changed();
    }

    /// Updates the spacer at `idx`, if the spacer was a fixed spacer, it will be overwritten with a flex spacer
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a spacer.
    pub fn update_spacer_flex(this: &mut WidgetMut<'_, Self>, idx: usize, flex: f64) {
        let child = &mut this.widget.children[idx];

        match *child {
            Child::FixedSpacer(_, _) | Child::FlexedSpacer(_, _) => {
                *child = Child::FlexedSpacer(flex, 0.0);
            }
            _ => {
                panic!("Can't update spacer parameters of a non-spacer element");
            }
        };
        this.ctx.children_changed();
    }

    /// Updates the spacer at `idx`, if the spacer was a flex spacer, it will be overwritten with a fixed spacer
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a spacer.
    pub fn update_spacer_fixed(this: &mut WidgetMut<'_, Self>, idx: usize, len: Length) {
        let child = &mut this.widget.children[idx];

        match *child {
            Child::FixedSpacer(_, _) | Child::FlexedSpacer(_, _) => {
                *child = Child::FixedSpacer(len, 0.0);
            }
            _ => {
                panic!("Can't update spacer parameters of a non-spacer element");
            }
        };
        this.ctx.children_changed();
    }

    /// Remove all children from the container.
    pub fn clear(this: &mut WidgetMut<'_, Self>) {
        if !this.widget.children.is_empty() {
            this.ctx.request_layout();

            for child in this.widget.children.drain(..) {
                if let Child::Fixed { widget, .. } | Child::Flex { widget, .. } = child {
                    this.ctx.remove_child(widget);
                }
            }
        }
    }
}

// --- MARK: OTHER IMPLS
impl FlexParams {
    /// Create custom `FlexParams` with a specific `flex_factor` and an optional
    /// [`CrossAxisAlignment`].
    ///
    /// You likely only need to create these manually if you need to specify
    /// a custom alignment; if you only need to use a custom `flex_factor` you
    /// can pass an `f64` to any of the functions that take `FlexParams`.
    ///
    /// By default, the widget uses the alignment of its parent [`Flex`] container.
    pub fn new(
        flex: impl Into<Option<f64>>,
        alignment: impl Into<Option<CrossAxisAlignment>>,
    ) -> Self {
        let flex = match flex.into() {
            Some(flex) if flex <= 0.0 => {
                debug_panic!("Flex value should be > 0.0. Flex given was: {}", flex);
                Some(0.0)
            }
            other => other,
        };

        Self {
            flex,
            alignment: alignment.into(),
        }
    }
}

impl From<f64> for FlexParams {
    fn from(flex: f64) -> Self {
        Self::new(flex, None)
    }
}

impl From<CrossAxisAlignment> for FlexParams {
    fn from(alignment: CrossAxisAlignment) -> Self {
        Self::new(None, alignment)
    }
}

impl Child {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<dyn Widget>> {
        match self {
            Self::Fixed { widget, .. } | Self::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
    fn widget(&self) -> Option<&WidgetPod<dyn Widget>> {
        match self {
            Self::Fixed { widget, .. } | Self::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
}

fn new_flex_child(params: FlexParams, child: WidgetPod<dyn Widget>) -> Child {
    if let Some(flex) = params.flex {
        if flex.is_normal() && flex > 0.0 {
            Child::Flex {
                widget: child,
                alignment: params.alignment,
                flex,
            }
        } else {
            tracing::warn!(
                "Flex value should be > 0.0 (was {flex}). See the docs for masonry::widgets::Flex for more information"
            );
            Child::Fixed {
                widget: child,
                alignment: params.alignment,
            }
        }
    } else {
        Child::Fixed {
            widget: child,
            alignment: params.alignment,
        }
    }
}

fn get_spacing(alignment: MainAxisAlignment, extra: f64, child_count: usize) -> (f64, f64) {
    let space_before;
    let space_between;
    match alignment {
        _ if child_count == 0 => {
            space_before = 0.;
            space_between = 0.;
        }
        MainAxisAlignment::Start => {
            space_before = 0.;
            space_between = 0.;
        }
        MainAxisAlignment::End => {
            space_before = extra;
            space_between = 0.;
        }
        MainAxisAlignment::Center => {
            space_before = extra / 2.;
            space_between = 0.;
        }
        MainAxisAlignment::SpaceBetween => {
            let equal_space = extra / (child_count - 1).max(1) as f64;
            space_before = 0.;
            space_between = equal_space;
        }
        MainAxisAlignment::SpaceEvenly => {
            let equal_space = extra / (child_count + 1) as f64;
            space_before = equal_space;
            space_between = equal_space;
        }
        MainAxisAlignment::SpaceAround => {
            let equal_space = extra / (2 * child_count) as f64;
            space_before = equal_space;
            space_between = equal_space * 2.;
        }
    }
    (space_before, space_between)
}

impl HasProperty<Background> for Flex {}
impl HasProperty<BorderColor> for Flex {}
impl HasProperty<BorderWidth> for Flex {}
impl HasProperty<CornerRadius> for Flex {}
impl HasProperty<Padding> for Flex {}

// --- MARK: IMPL WIDGET
impl Widget for Flex {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            ctx.register_child(child);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // SETUP
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        const MIN_FLEX_SUM: f64 = 0.0001;
        let gap_count = self.children.len().saturating_sub(1);
        let bc_major_min = self.direction.major(bc.min());
        let bc_major_max = self.direction.major(bc.max());

        // ACCUMULATORS
        let mut minor = self.direction.minor(bc.min());
        let mut major_non_flex = gap_count as f64 * self.gap.get();
        let mut major_flex: f64 = 0.0;
        // We start with a small value to avoid divide-by-zero errors.
        let mut flex_sum = MIN_FLEX_SUM;
        // Values used if any child has `CrossAxisAlignment::Baseline`.
        let mut max_above_baseline = 0_f64;
        let mut max_below_baseline = 0_f64;

        // MEASURE FIXED CHILDREN AND FLEX SUM
        for child in &mut self.children {
            match child {
                Child::Fixed { widget, .. } => {
                    // The BoxConstraints of fixed-children only depends on the BoxConstraints of the
                    // Flex widget.
                    let child_size = {
                        let child_size = ctx.run_layout(widget, &loosened_bc);

                        if child_size.width.is_infinite() {
                            tracing::warn!("A non-Flex child has an infinite width.");
                        }

                        if child_size.height.is_infinite() {
                            tracing::warn!("A non-Flex child has an infinite height.");
                        }

                        child_size
                    };

                    let baseline_offset = ctx.child_baseline_offset(widget);

                    major_non_flex += self.direction.major(child_size);
                    minor = minor.max(self.direction.minor(child_size));
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FixedSpacer(kv, calculated_size) => {
                    *calculated_size = kv.get();
                    if *calculated_size < 0.0 {
                        tracing::warn!("Length provided to fixed spacer was less than 0");
                    }
                    *calculated_size = calculated_size.max(0.0);
                    major_non_flex += *calculated_size;
                }
                Child::Flex { flex, .. } | Child::FlexedSpacer(flex, _) => flex_sum += *flex,
            }
        }

        let remaining_major = (bc_major_max - major_non_flex).max(0.0);
        let px_per_flex = remaining_major / flex_sum;

        // MEASURE FLEX CHILDREN
        for child in &mut self.children {
            match child {
                Child::Flex { widget, flex, .. } => {
                    let child_size = {
                        let desired_major = (*flex) * px_per_flex;

                        let child_bc = self.direction.constraints(&loosened_bc, 0.0, desired_major);
                        ctx.run_layout(widget, &child_bc)
                    };

                    let baseline_offset = ctx.child_baseline_offset(widget);

                    major_flex += self.direction.major(child_size);
                    minor = minor.max(self.direction.minor(child_size));
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FlexedSpacer(flex, calculated_size) => {
                    let desired_major = (*flex) * px_per_flex;
                    *calculated_size = desired_major;
                    major_flex += *calculated_size;
                }
                _ => {}
            }
        }

        // COMPUTE EXTRA SPACE
        let extra_length = if self.fill_major_axis {
            (remaining_major - major_flex).max(0.0)
        } else {
            // If we are *not* expected to fill our available space this usually
            // means we don't have any extra, unless dictated by our constraints.
            (self.direction.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
        };
        // We only distribute free space around widgets, not spacers.
        let widget_count = self
            .children
            .iter()
            .filter(|child| child.widget().is_some())
            .count();
        let (space_before, space_between) =
            get_spacing(self.main_alignment, extra_length, widget_count);

        // DISTRIBUTE EXTRA SPACE
        let mut major = space_before;
        let mut previous_was_widget = false;
        for child in &mut self.children {
            match child {
                Child::Fixed { widget, alignment }
                | Child::Flex {
                    widget, alignment, ..
                } => {
                    if previous_was_widget {
                        major += space_between;
                    }

                    let child_size = ctx.child_size(widget);
                    let alignment = alignment.unwrap_or(self.cross_alignment);
                    let child_minor_offset = match alignment {
                        CrossAxisAlignment::Baseline if self.direction == Axis::Horizontal => {
                            let max_height = max_below_baseline + max_above_baseline;
                            let extra_height = (minor - max_height).max(0.);

                            let child_baseline = ctx.child_baseline_offset(widget);
                            let child_above_baseline = child_size.height - child_baseline;
                            extra_height + (max_above_baseline - child_above_baseline)
                        }
                        CrossAxisAlignment::Fill => {
                            let fill_size: Size = self
                                .direction
                                .pack(self.direction.major(child_size), minor)
                                .into();
                            let child_bc = BoxConstraints::tight(fill_size);
                            // TODO: This is the second call of layout on the same child,
                            // which can lead to exponential increase in layout calls
                            // when used multiple times in the widget hierarchy.
                            ctx.run_layout(widget, &child_bc);
                            0.0
                        }
                        _ => {
                            let extra_minor = minor - self.direction.minor(child_size);
                            alignment.align(extra_minor)
                        }
                    };

                    let child_pos: Point = self.direction.pack(major, child_minor_offset).into();
                    let child_pos = border.place_down(child_pos);
                    let child_pos = padding.place_down(child_pos);
                    ctx.place_child(widget, child_pos);

                    major += self.direction.major(child_size);
                    major += self.gap.get();
                    previous_was_widget = true;
                }
                Child::FlexedSpacer(_, calculated_size)
                | Child::FixedSpacer(_, calculated_size) => {
                    major += *calculated_size;
                    major += self.gap.get();
                    previous_was_widget = false;
                }
            }
        }

        if flex_sum > MIN_FLEX_SUM && bc_major_max.is_infinite() {
            tracing::warn!("A child of Flex is flex, but Flex is unbounded.");
        }

        let final_major = if flex_sum > MIN_FLEX_SUM || self.fill_major_axis {
            bc_major_max.max(major_non_flex)
        } else {
            bc_major_min.max(major_non_flex)
        };

        let my_size: Size = self.direction.pack(final_major, minor).into();

        let baseline = match self.direction {
            Axis::Horizontal => max_below_baseline,
            Axis::Vertical => self
                .children
                .last()
                .map(|last| {
                    let child = last.widget();
                    if let Some(widget) = child {
                        let child_bl = ctx.child_baseline_offset(widget);
                        let child_max_y = ctx.child_layout_rect(widget).max_y();
                        let extra_bottom_padding = my_size.height - child_max_y;
                        child_bl + extra_bottom_padding
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0),
        };

        let (my_size, baseline) = padding.layout_up(my_size, baseline);
        let (my_size, baseline) = border.layout_up(my_size, baseline);
        ctx.set_baseline_offset(baseline);
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let bg = props.get::<Background>();
        let border_color = props.get::<BorderColor>();

        let bg_rect = border_width.bg_rect(ctx.size(), border_radius);
        let border_rect = border_width.border_rect(ctx.size(), border_radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);

        // paint the baseline if we're debugging layout
        if ctx.debug_paint_enabled() && ctx.baseline_offset() != 0.0 {
            let color = ctx.debug_color();
            let my_baseline = ctx.size().height - ctx.baseline_offset();
            let line = Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        self.children
            .iter()
            .filter_map(|child| child.widget())
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Flex", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::assert_debug_panics;

    use super::*;
    use crate::properties::types::AsUnit;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, test_property_set};
    use crate::widgets::Label;

    #[test]
    fn test_main_axis_alignment_spacing() {
        let apply_align = |align, extra, child_count| {
            let (space_before, space_between) = get_spacing(align, extra, child_count);
            let space_after =
                extra - space_before - space_between * child_count.saturating_sub(1) as f64;
            (space_before, space_between, space_after)
        };

        // Formatting note: in the comments below:
        // `[-]` represents a child.
        // a number represents a non-zero amount of space.

        let align = MainAxisAlignment::Start;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: [-] 10
        assert_eq!(before, 0.);
        assert_eq!(after, 10.);

        let (before, between, after) = apply_align(align, 10., 2);
        // Spacing: [-][-] 10
        assert_eq!(before, 0.);
        assert_eq!(between, 0.);
        assert_eq!(after, 10.);

        let align = MainAxisAlignment::End;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: 10 [-]
        assert_eq!(before, 10.);
        assert_eq!(after, 0.);

        let (before, between, after) = apply_align(align, 10., 2);
        // Spacing: 10 [-][-]
        assert_eq!(before, 10.);
        assert_eq!(between, 0.);
        assert_eq!(after, 0.);

        let align = MainAxisAlignment::Center;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: 5 [-] 5
        assert_eq!(before, 5.);
        assert_eq!(after, 5.);

        let (before, between, after) = apply_align(align, 10., 3);
        // Spacing: 5 [-][-][-] 5
        assert_eq!(before, 5.);
        assert_eq!(between, 0.);
        assert_eq!(after, 5.);

        let (before, between, after) = apply_align(align, 5., 2);
        // Spacing: 2.5 [-][-] 2.5
        assert_eq!(before, 2.5);
        assert_eq!(between, 0.);
        assert_eq!(after, 2.5);

        let align = MainAxisAlignment::SpaceBetween;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: [-] 10
        assert_eq!(before, 0.);
        assert_eq!(after, 10.);

        let (before, between, after) = apply_align(align, 10., 2);
        // Spacing: [-] 10 [-]
        assert_eq!(before, 0.);
        assert_eq!(between, 10.);
        assert_eq!(after, 0.);

        let (before, between, after) = apply_align(align, 30., 5);
        // Spacing: [-] 7.5 [-] 7.5 [-] 7.5 [-] 7.5 [-]
        assert_eq!(before, 0.);
        assert_eq!(between, 7.5);
        assert_eq!(after, 0.);

        let align = MainAxisAlignment::SpaceEvenly;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: 5 [-] 5
        assert_eq!(before, 5.);
        assert_eq!(after, 5.);

        let (before, between, after) = apply_align(align, 10., 3);
        // Spacing: 2.5 [-] 2.5 [-] 2.5 [-] 2.5
        assert_eq!(before, 2.5);
        assert_eq!(between, 2.5);
        assert_eq!(after, 2.5);

        let align = MainAxisAlignment::SpaceAround;
        let (before, _, after) = apply_align(align, 10., 1);
        // Spacing: 5 [-] 5
        assert_eq!(before, 5.);
        assert_eq!(after, 5.);

        let (before, between, after) = apply_align(align, 10., 2);
        // Spacing: 2.5 [-] 5 [-] 2.5
        assert_eq!(before, 2.5);
        assert_eq!(between, 5.);
        assert_eq!(after, 2.5);

        let (before, between, after) = apply_align(align, 35., 5);
        // Spacing: 3.5 [-] 7 [-] 7 [-] 7 [-] 7 [-] 3.5
        assert_eq!(before, 3.5);
        assert_eq!(between, 7.);
        assert_eq!(after, 3.5);
    }

    #[test]
    fn invalid_flex_params() {
        assert_debug_panics!(FlexParams::new(0.0, None), "Flex value should be > 0.0");
        assert_debug_panics!(FlexParams::new(-0.0, None), "Flex value should be > 0.0");
        assert_debug_panics!(FlexParams::new(-1.0, None), "Flex value should be > 0.0");
    }

    #[test]
    fn flex_row_fixed_size_only() {
        let widget = NewWidget::new_with_props(
            Flex::row()
                .with_child(Label::new("hello").with_auto_id())
                .with_child(Label::new("world").with_auto_id())
                .with_child(Label::new("foo").with_auto_id())
                .with_child(Label::new("bar").with_auto_id()),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
        );

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_start");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_center");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_end");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_spaceBetween");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_spaceEvenly");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "flex_row_fixed_children_spaceAround");
    }

    // TODO - Reduce copy-pasting?
    #[test]
    fn flex_row_cross_axis_snapshots() {
        let widget = NewWidget::new_with_props(
            Flex::row()
                .with_child(Label::new("hello").with_auto_id())
                .with_flex_child(Label::new("world").with_auto_id(), 1.0)
                .with_child(Label::new("foo").with_auto_id())
                .with_flex_child(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
        );

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_start");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_center");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_end");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_baseline");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_fill");
    }

    #[test]
    fn flex_row_main_axis_snapshots() {
        let widget = NewWidget::new_with_props(
            Flex::row()
                .with_child(Label::new("hello").with_auto_id())
                .with_flex_child(Label::new("world").with_auto_id(), 1.0)
                .with_child(Label::new("foo").with_auto_id())
                .with_flex_child(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
        );

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_start");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_center");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_end");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_spaceBetween");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "flex_row_main_axis_spaceAround");

        // FILL MAIN AXIS
        // TODO - This doesn't seem to do anything?

        harness.edit_root_widget(|mut flex| {
            Flex::set_must_fill_main_axis(&mut flex, true);
        });
        assert_render_snapshot!(harness, "flex_row_fill_main_axis");
    }

    #[test]
    fn flex_col_cross_axis_snapshots() {
        let widget = NewWidget::new_with_props(
            Flex::column()
                .with_child(Label::new("hello").with_auto_id())
                .with_flex_child(Label::new("world").with_auto_id(), 1.0)
                .with_child(Label::new("foo").with_auto_id())
                .with_flex_child(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
        );

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_start");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_center");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_end");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Baseline);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_baseline");

        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Fill);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_fill");
    }

    #[test]
    fn flex_col_main_axis_snapshots() {
        let widget = NewWidget::new_with_props(
            Flex::column()
                .with_child(Label::new("hello").with_auto_id())
                .with_flex_child(Label::new("world").with_auto_id(), 1.0)
                .with_child(Label::new("foo").with_auto_id())
                .with_flex_child(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
        );

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        // MAIN AXIS ALIGNMENT

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_start");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_center");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_end");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_spaceBetween");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_spaceEvenly");

        harness.edit_root_widget(|mut flex| {
            Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
        });
        assert_render_snapshot!(harness, "flex_col_main_axis_spaceAround");

        // FILL MAIN AXIS
        // TODO - This doesn't seem to do anything?

        harness.edit_root_widget(|mut flex| {
            Flex::set_must_fill_main_axis(&mut flex, true);
        });
        assert_render_snapshot!(harness, "flex_col_fill_main_axis");
    }

    #[test]
    fn edit_flex_container() {
        let image_1 = {
            let widget = Flex::column()
                .with_child(Label::new("a").with_auto_id())
                .with_child(Label::new("b").with_auto_id())
                .with_child(Label::new("c").with_auto_id())
                .with_child(Label::new("d").with_auto_id())
                .with_auto_id();
            // -> abcd

            let window_size = Size::new(200.0, 150.0);
            let mut harness =
                TestHarness::create_with_size(test_property_set(), widget, window_size);

            harness.edit_root_widget(|mut flex| {
                Flex::remove_child(&mut flex, 1);
                // -> acd
                Flex::add_child(&mut flex, Label::new("x").with_auto_id());
                // -> acdx
                Flex::add_flex_child(&mut flex, Label::new("y").with_auto_id(), 2.0);
                // -> acdxy
                Flex::add_spacer(&mut flex, 5.px());
                // -> acdxy_
                Flex::add_flex_spacer(&mut flex, 1.0);
                // -> acdxy__
                Flex::insert_child(&mut flex, 2, Label::new("i").with_auto_id());
                // -> acidxy__
                Flex::insert_flex_child(&mut flex, 2, Label::new("j").with_auto_id(), 2.0);
                // -> acjidxy__
                Flex::insert_spacer(&mut flex, 2, 5.px());
                // -> ac_jidxy__
                Flex::insert_flex_spacer(&mut flex, 2, 1.0);
                // -> ac__jidxy__
            });

            harness.render()
        };

        let image_2 = {
            let widget = Flex::column()
                .with_child(Label::new("a").with_auto_id())
                .with_child(Label::new("c").with_auto_id())
                .with_flex_spacer(1.0)
                .with_spacer(5.px())
                .with_flex_child(Label::new("j").with_auto_id(), 2.0)
                .with_child(Label::new("i").with_auto_id())
                .with_child(Label::new("d").with_auto_id())
                .with_child(Label::new("x").with_auto_id())
                .with_flex_child(Label::new("y").with_auto_id(), 2.0)
                .with_spacer(5.px())
                .with_flex_spacer(1.0)
                .with_auto_id();

            let window_size = Size::new(200.0, 150.0);
            let mut harness =
                TestHarness::create_with_size(test_property_set(), widget, window_size);
            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }

    #[test]
    fn get_flex_child() {
        let widget = Flex::column()
            .with_child(Label::new("hello").with_auto_id())
            .with_child(Label::new("world").with_auto_id())
            .with_spacer(1.px())
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        harness.edit_root_widget(|mut flex| {
            let mut child = Flex::child_mut(&mut flex, 1).unwrap();
            assert_eq!(
                child
                    .try_downcast::<Label>()
                    .unwrap()
                    .widget
                    .text()
                    .to_string(),
                "world"
            );
            drop(child);

            assert!(Flex::child_mut(&mut flex, 2).is_none());
        });

        // TODO - test out-of-bounds access?
    }

    #[test]
    fn divide_by_zero() {
        let widget = Flex::column().with_flex_spacer(0.0).with_auto_id();

        // Running layout should not panic when the flex sum is zero.
        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        harness.render();
    }
}
