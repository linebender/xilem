// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ChildrenIds, CollectionWidget, HasProperty, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::kurbo::{Affine, Axis, Line, Point, Size, Stroke};
use crate::layout::{LayoutSize, LenDef, LenReq, Length};
use crate::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use crate::properties::{BorderColor, BorderWidth, CornerRadius, Gap, Padding};
use crate::util::Sanitize;
use crate::util::stroke;

/// A container with either horizontal or vertical layout.
///
/// This widget is the foundation of most layouts, and is highly configurable.
///
/// Every child has a `Flex` specific configuration in the form of [`FlexParams`].
/// This configuration sets the flex factor, the basis, and the cross axis alignment.
///
/// The basis determines the starting size of each child. For fixed children this will default to
/// [`FlexBasis::Auto`] which means that they will be at their preferred size.
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
#[doc = concat!(
    "![Flex column with multiple labels](",
    include_doc_path!("screenshots/flex_col_main_axis_spaceAround.png"),
    ")",
)]
pub struct Flex {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    children: Vec<Child>,
}

/// The initial size of a [`Flex`] child before extra space distribution.
///
/// Children are ensured this initial size and if there is any extra space left,
/// that remaining space gets divided among the children based on their flex factors.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum FlexBasis {
    /// Automatically determine the basis based on how the child wants to be sized.
    ///
    /// If the child has no defined size, then its `MaxContent` will be measured.
    ///
    /// # Performance
    ///
    /// If used in combination with a non-zero flex factor it will cause
    /// an additional measurement pass on this child during [`Flex`]'s layout.
    #[default]
    Auto,
    /// Always use a zero basis for the child, regardless of its sizing wishes.
    Zero,
}

/// Optional parameters for an item in a [`Flex`] container (row or column).
///
/// Generally, when you would like to add a flexible child to a container,
/// you can simply call [`with`](Flex::with) or [`add`](Flex::add),
/// passing the child and the desired flex factor as a `f64`, which has an impl of
/// `Into<FlexParams>`.
///
/// The flex factor must be finite and non-negative.
///
/// You can also add spacers and flexible spacers using e.g. [`with_spacer`](Flex::with_spacer).
/// Spacers are children which take up space but don't paint anything.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct FlexParams {
    flex: f64,
    basis: Option<FlexBasis>,
    alignment: Option<CrossAxisAlignment>,
}

/// Returns the in-effect basis.
///
/// Either unwraps the given `basis`, or gives a reasonable default.
///
/// `FlexBasis::Auto` if `flex` is zero and `FlexBasis::Zero` otherwise.
fn effective_basis(basis: Option<FlexBasis>, flex: f64) -> FlexBasis {
    basis.unwrap_or(if flex == 0. {
        FlexBasis::Auto
    } else {
        FlexBasis::Zero
    })
}

// TODO: Remove these ephemeral scratch spaces, they are a real foot-gun.
//       Currently it's fine because we always write to them before reading.
//       However, having the compiler enforce that by using a local variable would be much better.
//       Problems arise if e.g. measure() writes and then layout() reads-before-writing.
enum Child {
    Widget {
        widget: WidgetPod<dyn Widget>,
        alignment: Option<CrossAxisAlignment>,
        flex: f64,
        basis: Option<FlexBasis>,
        /// Ephemeral resolved basis.
        ///
        /// It is a logic error to read this value before writing to it in the same method.
        basis_resolved: f64,
    },
    Spacer {
        flex: f64,
        basis: Length,
        /// Ephemeral resolved basis.
        ///
        /// It is a logic error to read this value before writing to it in the same method.
        basis_resolved: f64,
        /// Ephemeral resolved length.
        ///
        /// It is a logic error to read this value before writing to it in the same method.
        length_resolved: f64,
    },
}

// --- MARK: BUILDERS
impl Flex {
    /// Creates a new `Flex` oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Self {
            direction: axis,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
        }
    }

    /// Creates a new horizontal container.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    ///
    pub fn row() -> Self {
        Self::for_axis(Axis::Horizontal)
    }

    /// Creates a new vertical container.
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

    /// Builder-style variant of [`Flex::add_fixed`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_fixed(mut self, child: NewWidget<impl Widget + ?Sized>) -> Self {
        let child = new_child(0., child.erased().to_pod());
        self.children.push(child);
        self
    }

    /// Builder-style method to add a flexible child to the container.
    pub fn with(
        mut self,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) -> Self {
        let child = new_child(params, child.erased().to_pod());
        self.children.push(child);
        self
    }

    /// Builder-style method for adding a fixed-size spacer child to the container.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn with_fixed_spacer(mut self, len: Length) -> Self {
        let new_child = Child::Spacer {
            flex: 0.,
            basis: len,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        self.children.push(new_child);
        self
    }

    /// Builder-style method for adding a `flex` spacer child to the container.
    ///
    /// The `flex` factor must be finite and non-negative.
    /// Non-finite or negative flex factor will fall back to zero with a logged warning.
    ///
    /// # Panics
    ///
    /// Panics if `flex` is non-finite or negative and debug assertions are enabled.
    pub fn with_spacer(mut self, flex: f64) -> Self {
        let flex = flex.sanitize("spacer flex factor");
        let new_child = Child::Spacer {
            flex,
            basis: Length::ZERO,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        self.children.push(new_child);
        self
    }
}

// --- MARK: WIDGETMUT
impl Flex {
    /// Sets the flex direction (see [`Axis`]).
    pub fn set_direction(this: &mut WidgetMut<'_, Self>, direction: Axis) {
        this.widget.direction = direction;
        this.ctx.request_layout();
    }

    /// Sets the children's [`CrossAxisAlignment`].
    pub fn set_cross_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: CrossAxisAlignment) {
        this.widget.cross_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Sets the children's [`MainAxisAlignment`].
    pub fn set_main_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: MainAxisAlignment) {
        this.widget.main_alignment = alignment;
        this.ctx.request_layout();
    }

    /// Adds a non-flex child widget.
    ///
    /// See also [`with_fixed`].
    ///
    /// [`with_fixed`]: Flex::with_fixed
    pub fn add_fixed(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let child = new_child(0., child.erased().to_pod());
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Adds an empty spacer child with the given size.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn add_fixed_spacer(this: &mut WidgetMut<'_, Self>, len: Length) {
        let new_child = Child::Spacer {
            flex: 0.,
            basis: len,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        this.widget.children.push(new_child);
        this.ctx.request_layout();
    }

    /// Adds an empty spacer child with a specific `flex` factor.
    ///
    /// The `flex` factor must be finite and non-negative.
    /// Non-finite or negative flex factor will fall back to zero with a logged warning.
    ///
    /// # Panics
    ///
    /// Panics if `flex` is non-finite or negative and debug assertions are enabled.
    pub fn add_spacer(this: &mut WidgetMut<'_, Self>, flex: f64) {
        let flex = flex.sanitize("spacer flex factor");
        let new_child = Child::Spacer {
            flex,
            basis: Length::ZERO,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        this.widget.children.push(new_child);
        this.ctx.request_layout();
    }

    /// Inserts a non-flex child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    pub fn insert_fixed(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
    ) {
        let child = new_child(0., child.erased().to_pod());
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Inserts an empty spacer child with the given size at the given index.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    pub fn insert_fixed_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, len: Length) {
        let new_child = Child::Spacer {
            flex: 0.,
            basis: len,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        this.widget.children.insert(idx, new_child);
        this.ctx.request_layout();
    }

    /// Adds an empty spacer child with a specific `flex` factor.
    ///
    /// The `flex` factor must be finite and non-negative.
    /// Non-finite or negative flex factor will fall back to zero with a logged warning.
    ///
    /// # Panics
    ///
    /// Panics if `flex` is non-finite or negative and debug assertions are enabled.
    ///
    /// Panics if `idx` is larger than the number of children.
    pub fn insert_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, flex: f64) {
        let flex = flex.sanitize("spacer flex factor");
        let new_child = Child::Spacer {
            flex,
            basis: Length::ZERO,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        this.widget.children.insert(idx, new_child);
        this.ctx.request_layout();
    }

    /// Replaces the child widget at the given index with a new non-flex one.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub fn set_fixed(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
    ) {
        let child = new_child(0., child.erased().to_pod());
        let old_child = std::mem::replace(&mut this.widget.children[idx], child);
        if let Child::Widget { widget, .. } = old_child {
            this.ctx.remove_child(widget);
        }
        this.ctx.children_changed();
    }

    /// Replaces the child widget at the given index with an empty spacer.
    ///
    /// A good default is [`DEFAULT_SPACER_LEN`](crate::theme::DEFAULT_SPACER_LEN).
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub fn set_fixed_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, len: Length) {
        let new_child = Child::Spacer {
            flex: 0.,
            basis: len,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        let old_child = std::mem::replace(&mut this.widget.children[idx], new_child);
        if let Child::Widget { widget, .. } = old_child {
            this.ctx.remove_child(widget);
        }
        this.ctx.request_layout();
    }

    /// Replaces the child widget at the given index
    /// with an empty spacer with a specific `flex` factor.
    ///
    /// The `flex` factor must be finite and non-negative.
    /// Non-finite or negative flex factor will fall back to zero with a logged warning.
    ///
    /// # Panics
    ///
    /// Panics if `flex` is non-finite or negative and debug assertions are enabled.
    ///
    /// Panics if `idx` is out of bounds.
    pub fn set_spacer(this: &mut WidgetMut<'_, Self>, idx: usize, flex: f64) {
        let flex = flex.sanitize("spacer flex factor");
        let new_child = Child::Spacer {
            flex,
            basis: Length::ZERO,
            basis_resolved: 0.,
            length_resolved: 0.,
        };
        let old_child = std::mem::replace(&mut this.widget.children[idx], new_child);
        if let Child::Widget { widget, .. } = old_child {
            this.ctx.remove_child(widget);
        }
        this.ctx.request_layout();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<FlexParams> for Flex {
    /// Returns the number of children (widgets and spacers).
    fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if there are no children (widgets or spacers).
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a mutable reference to the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    ///
    /// Panics if `idx` contains a spacer instead of a widget.
    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let child = match &mut this.widget.children[idx] {
            Child::Widget { widget, .. } => widget,
            Child::Spacer { .. } => panic!("The provided Flex idx contains a spacer"),
        };
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) {
        let child = child.erased().to_pod();
        let child = new_child(params, child);

        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Inserts a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) {
        let child = child.erased().to_pod();
        let child = new_child(params, child);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Replaces the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<FlexParams>,
    ) {
        let child = child.erased().to_pod();
        let child = new_child(params, child);
        let old_child = std::mem::replace(&mut this.widget.children[idx], child);
        if let Child::Widget { widget, .. } = old_child {
            this.ctx.remove_child(widget);
        }
        this.ctx.children_changed();
    }

    /// Sets the child params at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    ///
    /// Panics if `idx` contains a spacer instead of a widget.
    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<FlexParams>) {
        let child = &mut this.widget.children[idx];
        let child_val = std::mem::replace(
            child,
            Child::Spacer {
                flex: 0.,
                basis: Length::ZERO,
                basis_resolved: 0.,
                length_resolved: 0.,
            },
        );
        let widget = match child_val {
            Child::Widget { widget, .. } => widget,
            Child::Spacer { .. } => {
                panic!("Can't update flex parameters of a spacer element");
            }
        };
        let new_child = new_child(params, widget);
        *child = new_child;
        this.ctx.children_changed();
    }

    /// Swaps the index of two children.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        this.widget.children.swap(a, b);
        this.ctx.children_changed();
    }

    /// Removes the child at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        if let Child::Widget { widget, .. } = child {
            this.ctx.remove_child(widget);
        } else {
            // We need to explicitly request layout in case of spacer removal
            this.ctx.request_layout();
        }
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        if !this.widget.children.is_empty() {
            for child in this.widget.children.drain(..) {
                if let Child::Widget { widget, .. } = child {
                    this.ctx.remove_child(widget);
                }
            }
            // We need to explicitly request layout in case we had any spacers
            this.ctx.request_layout();
        }
    }
}

// --- MARK: OTHER IMPLS
impl FlexParams {
    /// Creates custom `FlexParams` with a specific `flex` factor,
    /// and optionally with [`FlexBasis`] and [`CrossAxisAlignment`].
    ///
    /// You likely only need to create these manually if you need to specify
    /// a custom basis or alignment; if you only need to use a custom `flex` factor you
    /// can pass an `f64` to any of the functions that take `FlexParams`.
    ///
    /// The `flex` factor must be finite and non-negative.
    /// Non-finite or negative flex factor will fall back to zero with a logged warning.
    ///
    /// By default, the widget uses either [`FlexBasis::Auto`] or [`FlexBasis::Zero`],
    /// depending on whether the flex factor is zero or not.
    ///
    /// By default, the widget uses the alignment of its parent [`Flex`] container.
    ///
    /// # Panics
    ///
    /// Panics if `flex` is non-finite or negative and debug assertions are enabled.
    pub fn new(
        flex: f64,
        basis: impl Into<Option<FlexBasis>>,
        alignment: impl Into<Option<CrossAxisAlignment>>,
    ) -> Self {
        let flex = flex.sanitize("flex factor");
        Self {
            flex,
            basis: basis.into(),
            alignment: alignment.into(),
        }
    }
}

impl From<f64> for FlexParams {
    fn from(flex: f64) -> Self {
        Self::new(flex, None, None)
    }
}

impl From<CrossAxisAlignment> for FlexParams {
    fn from(alignment: CrossAxisAlignment) -> Self {
        Self {
            alignment: Some(alignment),
            ..Default::default()
        }
    }
}

impl Child {
    fn is_widget(&self) -> bool {
        matches!(self, Self::Widget { .. })
    }

    fn widget_mut(&mut self) -> Option<&mut WidgetPod<dyn Widget>> {
        match self {
            Self::Widget { widget, .. } => Some(widget),
            _ => None,
        }
    }

    fn widget(&self) -> Option<&WidgetPod<dyn Widget>> {
        match self {
            Self::Widget { widget, .. } => Some(widget),
            _ => None,
        }
    }
}

/// Creates a new [`Child::Widget`].
fn new_child(params: impl Into<FlexParams>, child: WidgetPod<dyn Widget>) -> Child {
    let params = params.into();
    Child::Widget {
        widget: child,
        alignment: params.alignment,
        flex: params.flex,
        basis: params.basis,
        basis_resolved: 0.,
    }
}

/// Calculates `(space_before, space_between)` from the `extra` space given the `child_count`.
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

impl HasProperty<BorderColor> for Flex {}
impl HasProperty<BorderWidth> for Flex {}
impl HasProperty<CornerRadius> for Flex {}
impl HasProperty<Gap> for Flex {}

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
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Gap::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        // The usual axis input has been named measure_axis here,
        // to make it harder to use it in the wrong context by accident.
        measure_axis: Axis,
        len_req: LenReq,
        // The usual cross_length input has been named perp_length here,
        // to remove the collision with flex cross, which might not match.
        perp_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let perp = measure_axis.cross();
        let main = self.direction;
        let cross = main.cross();

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let gap = props.get::<Gap>();

        let border_length = border.length(measure_axis).dp(scale);
        let padding_length = padding.length(measure_axis).dp(scale);

        let gap_length = gap.gap.dp(scale);
        let gap_count = self.children.len().saturating_sub(1);

        let perp_space = perp_length.map(|perp_length| {
            let prep_border_length = border.length(perp).dp(scale);
            let prep_padding_length = padding.length(perp).dp(scale);
            (perp_length - prep_border_length - prep_padding_length).max(0.)
        });
        let (main_space, cross_space) = if perp == main {
            (perp_space, None)
        } else {
            (None, perp_space)
        };
        let context_size = LayoutSize::maybe(perp, perp_space);

        let (len_req, min_result) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (len_req, 0.),
            // We always want to use up all offered space but may need even more,
            // so we implement FitContent as space.max(MinContent).
            LenReq::FitContent(space) => (LenReq::MinContent, space),
        };

        // We can skip resolving bases if we don't know the main space when measuring cross.
        // That is because in that code path we don't ever read the resolved basis.
        let skip_resolving_bases = measure_axis == cross && main_space.is_none();

        // Resolve bases
        if !skip_resolving_bases {
            // Basis is always resolved with a MaxContent fallback
            let main_auto = LenDef::MaxContent;

            for child in &mut self.children {
                match child {
                    Child::Widget {
                        widget,
                        flex,
                        basis,
                        basis_resolved,
                        ..
                    } => match effective_basis(*basis, *flex) {
                        FlexBasis::Auto => {
                            *basis_resolved = ctx.compute_length(
                                widget,
                                main_auto,
                                context_size,
                                main,
                                cross_space,
                            );
                        }
                        FlexBasis::Zero => {
                            // TODO: When min/max constraints become a real thing,
                            //      then need to account for them here.
                            *basis_resolved = 0.;
                        }
                    },
                    Child::Spacer {
                        basis,
                        basis_resolved,
                        ..
                    } => {
                        *basis_resolved = basis.dp(scale);
                    }
                }
            }
        }

        let mut length = 0.;
        if measure_axis == main {
            // Calculate the main axis length

            // Find the largest desired flex fraction
            let mut flex_fraction: f64 = 0.;
            let main_auto = len_req.into();

            for child in &mut self.children {
                let desired_flex_fraction = match child {
                    Child::Widget {
                        widget,
                        flex,
                        basis,
                        ..
                    } => {
                        if *flex > 0. {
                            match effective_basis(*basis, *flex) {
                                FlexBasis::Auto => {
                                    // Auto basis is always MaxContent, so this child doesn't want
                                    // any extra flex space regardless if the request is Min or Max.
                                    0.
                                }
                                FlexBasis::Zero => {
                                    let child_length = ctx.compute_length(
                                        widget,
                                        main_auto,
                                        context_size,
                                        main,
                                        cross_space,
                                    );
                                    // Flexible children with a zero basis want to reach
                                    // their target length purely with flex space.
                                    child_length / *flex
                                }
                            }
                        } else {
                            // Inflexible children remain at their basis size,
                            // and don't want any extra flex space.
                            0.
                        }
                    }
                    Child::Spacer { .. } => {
                        // Spacer basis fully covers its preferred size,
                        // so spacers don't want any extra flex space.
                        0.
                    }
                };
                flex_fraction = flex_fraction.max(desired_flex_fraction);
            }

            // Calculate the total space needed for all children
            length += self
                .children
                .iter()
                .map(|child| match child {
                    Child::Widget {
                        flex,
                        basis_resolved,
                        ..
                    }
                    | Child::Spacer {
                        flex,
                        basis_resolved,
                        ..
                    } => *basis_resolved + *flex * flex_fraction,
                })
                .sum::<f64>();

            // Add all the gap lengths
            length += gap_count as f64 * gap_length;
        } else {
            // Calculate the cross axis length

            // If we know the main axis space then we can distribute it to children.
            // This is important, because some widgets need it for accurate measurement.
            // For example text uses it to set max advance.
            let flex_fraction = main_space.map(|mut main_space| {
                // Sum flex factors and subtract bases from main space.
                let mut flex_sum = 0.;
                for child in &mut self.children {
                    match child {
                        Child::Widget {
                            flex,
                            basis_resolved,
                            ..
                        }
                        | Child::Spacer {
                            flex,
                            basis_resolved,
                            ..
                        } => {
                            flex_sum += *flex;
                            main_space -= *basis_resolved;
                        }
                    }
                }

                // Subtract gap lengths
                main_space -= gap_count as f64 * gap_length;

                // Calculate the flex fraction, i.e. the amount of space per one flex factor
                if flex_sum > 0. {
                    main_space.max(0.) / flex_sum
                } else {
                    0.
                }
            });

            // Calculate the total space needed for all children
            for child in &mut self.children {
                match child {
                    Child::Widget {
                        widget,
                        flex,
                        basis_resolved,
                        ..
                    } => {
                        let child_main_length = flex_fraction
                            .map(|flex_fraction| *basis_resolved + *flex * flex_fraction);
                        let cross_auto = len_req.into();

                        let child_cross_length = ctx.compute_length(
                            widget,
                            cross_auto,
                            context_size,
                            cross,
                            child_main_length,
                        );

                        length = length.max(child_cross_length);
                    }
                    // Spacers don't contribute to cross length
                    Child::Spacer { .. } => (),
                }
            }

            // Gaps don't contribute to the cross axis
        }

        min_result.max(length + border_length + padding_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        let gap = props.get::<Gap>();
        let gap_length = gap.gap.dp(scale);
        let gap_count = self.children.len().saturating_sub(1);

        let main = self.direction;
        let cross = main.cross();
        let cross_space = space.get_coord(cross);

        let mut main_space = space.get_coord(main) - gap_count as f64 * gap_length;
        let mut flex_sum = 0.;
        let mut max_ascent: f64 = 0.;
        let mut lowest_baseline: f64 = f64::INFINITY;

        // Helper function to calculate child size when main length is decided
        let compute_child_size =
            |ctx: &mut LayoutCtx<'_>,
             child: &mut WidgetPod<dyn Widget + 'static>,
             child_main_length: f64,
             alignment: &Option<CrossAxisAlignment>| {
                let cross_auto = match alignment.unwrap_or(self.cross_alignment) {
                    // Cross stretch is merely an auto fallback, not an immediate choice.
                    // That means that an explicit child length will override it, matching web.
                    CrossAxisAlignment::Stretch => LenDef::Fixed(cross_space),
                    _ => LenDef::FitContent(cross_space),
                };

                let child_cross_length = ctx.compute_length(
                    child,
                    cross_auto,
                    space.into(),
                    cross,
                    Some(child_main_length),
                );

                main.pack_size(child_main_length, child_cross_length)
            };

        // Helper function to lay out children
        let mut lay_out_child = |ctx: &mut LayoutCtx<'_>,
                                 child: &mut WidgetPod<dyn Widget + 'static>,
                                 child_size: Size| {
            ctx.run_layout(child, child_size);

            let baseline = ctx.child_baseline_offset(child);
            let ascent = child_size.height - baseline;
            max_ascent = max_ascent.max(ascent);
        };

        // Helper function to place children
        let mut place_child = |ctx: &mut LayoutCtx<'_>,
                               child: &mut WidgetPod<dyn Widget + 'static>,
                               child_origin: Point| {
            ctx.place_child(child, child_origin);

            let child_baseline = ctx.child_baseline_offset(child);
            let child_size = ctx.child_size(child);
            let child_bottom = child_origin.y + child_size.height;
            let bottom_gap = size.height - child_bottom;
            let baseline = child_baseline + bottom_gap;
            lowest_baseline = lowest_baseline.min(baseline);
        };

        // Sum flex factors, resolve bases, subtract bases from main space,
        // and lay out inflexible widgets.
        for child in &mut self.children {
            match child {
                Child::Widget {
                    widget,
                    alignment,
                    flex,
                    basis,
                    basis_resolved,
                } => {
                    match effective_basis(*basis, *flex) {
                        FlexBasis::Auto => {
                            // Basis is always resolved with a MaxContent fallback
                            let main_auto = LenDef::MaxContent;
                            *basis_resolved = ctx.compute_length(
                                widget,
                                main_auto,
                                space.into(),
                                main,
                                Some(cross_space),
                            );
                            main_space -= *basis_resolved;
                        }
                        FlexBasis::Zero => {
                            // TODO: When min/max constraints become a real thing,
                            //      then need to account for them here, and also
                            //      subtract the result for main_space.
                            *basis_resolved = 0.;
                        }
                    }
                    if *flex == 0. {
                        let child_main_length = *basis_resolved;
                        let child_size =
                            compute_child_size(ctx, widget, child_main_length, alignment);

                        lay_out_child(ctx, widget, child_size);
                    } else {
                        flex_sum += *flex;
                    }
                }
                Child::Spacer {
                    flex,
                    basis,
                    basis_resolved,
                    length_resolved,
                } => {
                    *basis_resolved = basis.dp(scale);
                    main_space -= *basis_resolved;

                    if *flex == 0. {
                        *length_resolved = *basis_resolved;
                    } else {
                        flex_sum += *flex;
                    }
                }
            }
        }

        // Calculate the flex fraction, i.e. the amount of space per one flex factor
        let flex_fraction = if flex_sum > 0. {
            main_space.max(0.) / flex_sum
        } else {
            0.
        };

        // Offer the available space to flexible children
        for child in &mut self.children {
            match child {
                Child::Widget {
                    widget,
                    alignment,
                    flex,
                    basis_resolved,
                    ..
                } if *flex > 0. => {
                    // Currently we just decide the space distribution in one go.
                    // When Flex gets configurable grow/shrink support,
                    // and min/max style constraints get implemented,
                    // this distribution will need to evolve into a looped solver.
                    let child_main_length = *basis_resolved + *flex * flex_fraction;
                    let child_size = compute_child_size(ctx, widget, child_main_length, alignment);

                    lay_out_child(ctx, widget, child_size);

                    main_space -= child_main_length - *basis_resolved;
                }
                Child::Spacer {
                    flex,
                    basis_resolved,
                    length_resolved,
                    ..
                } if *flex > 0. => {
                    let child_main_length = *basis_resolved + *flex * flex_fraction;
                    *length_resolved = child_main_length;
                    main_space -= *length_resolved - *basis_resolved;
                }
                _ => (),
            }
        }

        // We only distribute free space around widgets, not spacers.
        let widget_count = self
            .children
            .iter()
            .filter(|child| child.is_widget())
            .count();
        let (space_before, space_between) =
            get_spacing(self.main_alignment, main_space.max(0.), widget_count);

        // Distribute free space and place children
        let mut main_offset = space_before;
        let mut previous_was_widget = false;
        for child in &mut self.children {
            match child {
                Child::Widget {
                    widget, alignment, ..
                } => {
                    if previous_was_widget {
                        main_offset += space_between;
                    }

                    let child_size = ctx.child_size(widget);
                    let alignment = alignment.unwrap_or(self.cross_alignment);
                    let child_origin_cross = match alignment {
                        CrossAxisAlignment::Baseline if main == Axis::Horizontal => {
                            let baseline = ctx.child_baseline_offset(widget);
                            let ascent = child_size.height - baseline;
                            max_ascent - ascent
                        }
                        _ => {
                            let cross_unused = cross_space - child_size.get_coord(cross);
                            alignment.offset(cross_unused)
                        }
                    };

                    let child_origin = main.pack_point(main_offset, child_origin_cross);
                    let child_origin = border.origin_down(child_origin, scale);
                    let child_origin = padding.origin_down(child_origin, scale);
                    place_child(ctx, widget, child_origin);

                    main_offset += child_size.get_coord(main);
                    main_offset += gap_length;
                    previous_was_widget = true;
                }
                Child::Spacer {
                    length_resolved, ..
                } => {
                    main_offset += *length_resolved;
                    main_offset += gap_length;
                    previous_was_widget = false;
                }
            }
        }

        // If we have at least one child widget then we can use the lowest child baseline.
        let baseline = self
            .children
            .iter()
            .any(|child| child.is_widget())
            .then_some(lowest_baseline);

        ctx.set_baseline_offset(baseline.unwrap_or(0.));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let border_color = props.get::<BorderColor>();

        let border_rect = border_width.border_rect(ctx.size(), border_radius);

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
    use crate::layout::AsUnit;
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
        assert_debug_panics!(
            FlexParams::new(f64::NAN, None, None),
            "flex factor must be finite. Received: NaN"
        );
        assert_debug_panics!(
            FlexParams::new(f64::INFINITY, None, None),
            "flex factor must be finite. Received: inf"
        );
        assert_debug_panics!(
            FlexParams::new(-0.5, None, None),
            "flex factor must be non-negative. Received: -0.5"
        );
        assert_debug_panics!(
            FlexParams::new(-1.0, None, None),
            "flex factor must be non-negative. Received: -1"
        );
    }

    #[test]
    fn flex_row_fixed_size_only() {
        let widget = NewWidget::new_with_props(
            Flex::row()
                .with_fixed(Label::new("hello").with_auto_id())
                .with_fixed(Label::new("world").with_auto_id())
                .with_fixed(Label::new("foo").with_auto_id())
                .with_fixed(Label::new("bar").with_auto_id()),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)),
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
                .with_fixed(Label::new("hello").with_auto_id())
                .with(Label::new("world").with_auto_id(), 1.0)
                .with_fixed(Label::new("foo").with_auto_id())
                .with(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, None, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)),
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

        // TODO: Stretch with text doesn't make sense, it's not visible,
        //       unless we paint borders or background for the Label widget.
        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Stretch);
        });
        assert_render_snapshot!(harness, "flex_row_cross_axis_stretch");
    }

    #[test]
    fn flex_row_main_axis_snapshots() {
        // ALl children need to be fixed, otherwise a flexible child will use up all the space.
        let widget = NewWidget::new_with_props(
            Flex::row()
                .with_fixed(Label::new("hello").with_auto_id())
                .with_fixed(Label::new("world").with_auto_id())
                .with_fixed(Label::new("foo").with_auto_id())
                .with(Label::new("bar").with_auto_id(), CrossAxisAlignment::Start),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)),
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
    }

    #[test]
    fn flex_col_cross_axis_snapshots() {
        let widget = NewWidget::new_with_props(
            Flex::column()
                .with_fixed(Label::new("hello").with_auto_id())
                .with(Label::new("world").with_auto_id(), 1.0)
                .with_fixed(Label::new("foo").with_auto_id())
                .with(
                    Label::new("bar").with_auto_id(),
                    FlexParams::new(2.0, None, CrossAxisAlignment::Start),
                ),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)),
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

        // TODO: Stretch with text doesn't make sense, it's not visible,
        //       unless we paint borders or background for the Label widget.
        harness.edit_root_widget(|mut flex| {
            Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Stretch);
        });
        assert_render_snapshot!(harness, "flex_col_cross_axis_stretch");
    }

    #[test]
    fn flex_col_main_axis_snapshots() {
        // ALl children need to be fixed, otherwise a flexible child will use up all the space.
        let widget = NewWidget::new_with_props(
            Flex::column()
                .with_fixed(Label::new("hello").with_auto_id())
                .with_fixed(Label::new("world").with_auto_id())
                .with_fixed(Label::new("foo").with_auto_id())
                .with(Label::new("bar").with_auto_id(), CrossAxisAlignment::Start),
            (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)),
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
    }

    #[test]
    fn edit_flex_container() {
        let window_size = Size::new(50.0, 300.0);

        let image_1 = {
            let widget = Flex::column()
                .with_fixed(Label::new("q").with_auto_id())
                .with_fixed(Label::new("b").with_auto_id())
                .with_fixed(Label::new("w").with_auto_id())
                .with_fixed(Label::new("d").with_auto_id())
                .with_auto_id();
            // -> qbwd

            let mut harness =
                TestHarness::create_with_size(test_property_set(), widget, window_size);

            harness.edit_root_widget(|mut flex| {
                Flex::set_fixed(&mut flex, 0, Label::new("a").with_auto_id());
                // -> abwd
                Flex::set(&mut flex, 2, Label::new("c").with_auto_id(), 0.);
                // -> abcd
                Flex::remove(&mut flex, 1);
                // -> acd
                Flex::add_fixed(&mut flex, Label::new("x").with_auto_id());
                // -> acdx
                Flex::add(&mut flex, Label::new("y").with_auto_id(), 2.0);
                // -> acdxy
                Flex::add_fixed_spacer(&mut flex, 5.px());
                // -> acdxy_
                Flex::add_spacer(&mut flex, 1.0);
                // -> acdxy__
                Flex::insert_fixed(&mut flex, 2, Label::new("i").with_auto_id());
                // -> acidxy__
                Flex::insert(&mut flex, 2, Label::new("j").with_auto_id(), 2.0);
                // -> acjidxy__
                Flex::insert_fixed_spacer(&mut flex, 2, 5.px());
                // -> ac_jidxy__
                Flex::insert_spacer(&mut flex, 2, 1.0);
                // -> ac__jidxy__
            });

            harness.render()
        };

        let image_2 = {
            let widget = Flex::column()
                .with_fixed(Label::new("a").with_auto_id())
                .with_fixed(Label::new("c").with_auto_id())
                .with_spacer(1.0)
                .with_fixed_spacer(5.px())
                .with(Label::new("j").with_auto_id(), 2.0)
                .with_fixed(Label::new("i").with_auto_id())
                .with_fixed(Label::new("d").with_auto_id())
                .with_fixed(Label::new("x").with_auto_id())
                .with(Label::new("y").with_auto_id(), 2.0)
                .with_fixed_spacer(5.px())
                .with_spacer(1.0)
                .with_auto_id();

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
            .with_fixed(Label::new("hello").with_auto_id())
            .with_fixed(Label::new("world").with_auto_id())
            .with_fixed_spacer(1.px())
            .with_auto_id();

        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        harness.edit_root_widget(|mut flex| {
            let mut child = Flex::get_mut(&mut flex, 1);
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
        });

        // TODO - test out-of-bounds access?
    }

    #[test]
    fn divide_by_zero() {
        let widget = Flex::column().with_spacer(0.0).with_auto_id();

        // Running layout should not panic when the flex sum is zero.
        let window_size = Size::new(200.0, 150.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);
        harness.render();
    }
}
