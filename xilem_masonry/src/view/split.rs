// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::kurbo::Axis;
use masonry::layout::{AsUnit, Length};
use masonry::widgets;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use crate::{Pod, ViewCtx, WidgetView};

/// A container containing two other widgets, splitting the area either horizontally or vertically.
///
/// The split view will by default be draggable,
/// which means that the relative size of each child view can be changed by dragging the mouse.
///
/// # Examples
/// To create a split view, provide it with two child views.
///
/// ```ignore
/// # use xilem_masonry as xilem;
/// use xilem::view::{split, label};
///
/// split(
///     label("Left view"),
///     label("Right view")
/// )
/// ```
///
/// The split axis and split point can be changed.
/// For the full list of modifiers see the [`Split`] struct.
///
/// ```ignore
/// # use xilem_masonry as xilem;
/// use xilem::view::{split, label};
///
/// split(label("Left view"), label("Right view"))
///     .split_axis(Axis::Horizontal)
///     .split_point(0.25)
/// ```
///
pub fn split<State, Action, ChildA, ChildB>(
    child1: ChildA,
    child2: ChildB,
) -> Split<ChildA, ChildB, State, Action>
where
    ChildA: WidgetView<State, Action>,
    ChildB: WidgetView<State, Action>,
    State: 'static,
{
    Split {
        split_axis: Axis::Horizontal,
        split_point: widgets::SplitPoint::Fraction(0.5),
        min_lengths: (Length::ZERO, Length::ZERO),
        bar_thickness: 6.px(),
        min_bar_area: 6.px(),
        solid_bar: false,
        draggable: true,
        child1,
        child2,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`split`].
///
/// See `split` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Split<ChildA, ChildB, State, Action = ()> {
    split_axis: Axis,
    split_point: widgets::SplitPoint,
    min_lengths: (Length, Length),
    bar_thickness: Length,
    min_bar_area: Length,
    solid_bar: bool,
    draggable: bool,
    child1: ChildA,
    child2: ChildB,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<ChildA, ChildB, State, Action> Split<ChildA, ChildB, State, Action> {
    /// Set the split axis.
    ///
    /// Horizontal split axis means that the children are left and right.
    /// Vertical split axis means that the children are up and down.
    ///
    /// The default split point is horizontal.
    pub fn split_axis(mut self, axis: Axis) -> Self {
        self.split_axis = axis;
        self
    }

    /// Set the split point as a fraction of the split axis.
    ///
    /// The value is clamped to `0.0..=1.0`.
    /// The default split point is `0.5`.
    pub fn split_point(mut self, split_point: f64) -> Self {
        self.split_point = widgets::SplitPoint::Fraction(split_point.clamp(0.0, 1.0));
        self
    }

    /// Set the split point as an absolute distance from the start.
    ///
    /// This is the size of the first child along the split axis.
    pub fn split_point_from_start(mut self, split_point: Length) -> Self {
        self.split_point = widgets::SplitPoint::FromStart(split_point);
        self
    }

    /// Set the split point as an absolute distance from the end.
    ///
    /// This is the size of the second child along the split axis.
    pub fn split_point_from_end(mut self, split_point: Length) -> Self {
        self.split_point = widgets::SplitPoint::FromEnd(split_point);
        self
    }

    /// Set the split point.
    pub fn with_split_point(mut self, split_point: widgets::SplitPoint) -> Self {
        self.split_point = split_point;
        self
    }

    /// Set the minimum lengths for both sides of the split axis in logical pixels.
    pub fn min_lengths(mut self, first: Length, second: Length) -> Self {
        self.min_lengths = (first, second);
        self
    }

    /// Set the thickness of the splitter bar in logical pixels.
    ///
    /// The default splitter bar thickness is `6.0`.
    #[track_caller]
    pub fn bar_thickness(mut self, bar_thickness: Length) -> Self {
        self.bar_thickness = bar_thickness;
        self
    }

    /// Set the minimum thickness of the splitter bar area in logical pixels.
    ///
    /// The minimum splitter bar area defines the minimum thickness of the area
    /// where pointer hit detection is done for the splitter bar.
    /// The final hit detection area thickness is either this minimum
    /// or the splitter bar thickness, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The default minimum splitter bar area thickness is `6.0`.
    #[track_caller]
    pub fn min_bar_area(mut self, min_bar_area: Length) -> Self {
        self.min_bar_area = min_bar_area;
        self
    }

    /// Set whether the split point can be changed by dragging.
    ///
    /// The default is false.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Set whether the splitter bar is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), the bar will be drawn as two parallel lines.
    pub fn solid_bar(mut self, solid: bool) -> Self {
        self.solid_bar = solid;
        self
    }
}

// Use a distinctive number here, to be able to catch bugs.
// These were selected based on a random multiple (less than 1000) of 40960000.
// That base is chosen so that there are at least three trailing zeroes in both the hex
// and decimal forms, making the +1 obvious.

/// This is a randomly generated ID - 27361280000 in decimal.
const CHILD1_VIEW_ID: ViewId = ViewId::new(0x65edc0000);
/// This is a randomly generated ID - 27361280001 in decimal.
const CHILD2_VIEW_ID: ViewId = ViewId::new(0x65edc0001);

impl<ChildA, ChildB, State, Action> ViewMarker for Split<ChildA, ChildB, State, Action> {}
impl<ChildA, ChildB, State, Action> View<State, Action, ViewCtx>
    for Split<ChildA, ChildB, State, Action>
where
    State: 'static,
    Action: 'static,
    ChildA: WidgetView<State, Action>,
    ChildB: WidgetView<State, Action>,
{
    type Element = Pod<widgets::Split<ChildA::Widget, ChildB::Widget>>;

    type ViewState = (ChildA::ViewState, ChildB::ViewState);

    fn build(
        &self,
        ctx: &mut ViewCtx,
        mut app_state: &mut State,
    ) -> (Self::Element, Self::ViewState) {
        let (child1, child1_state) =
            ctx.with_id(CHILD1_VIEW_ID, |ctx| self.child1.build(ctx, &mut app_state));
        let (child2, child2_state) =
            ctx.with_id(CHILD2_VIEW_ID, |ctx| self.child2.build(ctx, &mut app_state));

        let widget_pod = ctx.create_pod(
            widgets::Split::new(child1.new_widget, child2.new_widget)
                .split_axis(self.split_axis)
                .split_point(self.split_point)
                .min_lengths(self.min_lengths.0, self.min_lengths.1)
                .bar_thickness(self.bar_thickness)
                .min_bar_area(self.min_bar_area)
                .draggable(self.draggable)
                .solid_bar(self.solid_bar),
        );

        (widget_pod, (child1_state, child2_state))
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        mut app_state: &mut State,
    ) {
        if prev.split_axis != self.split_axis {
            widgets::Split::set_split_axis(&mut element, self.split_axis);
        }

        if prev.split_point != self.split_point {
            widgets::Split::set_split_point(&mut element, self.split_point);
        }

        if prev.min_lengths != self.min_lengths {
            widgets::Split::set_min_lengths(&mut element, self.min_lengths.0, self.min_lengths.1);
        }

        if prev.bar_thickness != self.bar_thickness {
            widgets::Split::set_bar_thickness(&mut element, self.bar_thickness);
        }

        if prev.min_bar_area != self.min_bar_area {
            widgets::Split::set_min_bar_area(&mut element, self.min_bar_area);
        }

        if prev.draggable != self.draggable {
            widgets::Split::set_draggable(&mut element, self.draggable);
        }

        if prev.solid_bar != self.solid_bar {
            widgets::Split::set_bar_solid(&mut element, self.solid_bar);
        }

        ctx.with_id(CHILD1_VIEW_ID, |ctx| {
            let child1_element = widgets::Split::child1_mut(&mut element);
            self.child1.rebuild(
                &prev.child1,
                &mut view_state.0,
                ctx,
                child1_element,
                &mut app_state,
            );
        });

        ctx.with_id(CHILD2_VIEW_ID, |ctx| {
            let child2_element = widgets::Split::child2_mut(&mut element);
            self.child2.rebuild(
                &prev.child2,
                &mut view_state.1,
                ctx,
                child2_element,
                &mut app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let child1_element = widgets::Split::child1_mut(&mut element);
        self.child1.teardown(&mut view_state.0, ctx, child1_element);

        let child2_element = widgets::Split::child2_mut(&mut element);
        self.child2.teardown(&mut view_state.1, ctx, child2_element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(CHILD1_VIEW_ID) => {
                let child1_element = widgets::Split::child1_mut(&mut element);
                self.child1
                    .message(&mut view_state.0, message, child1_element, app_state)
            }
            Some(CHILD2_VIEW_ID) => {
                let child2_element = widgets::Split::child2_mut(&mut element);
                self.child2
                    .message(&mut view_state.1, message, child2_element, app_state)
            }
            view_id => {
                tracing::error!(
                    ?message,
                    "Invalid message arrived in Split::message, expected {:?} or {:?}, got {:?}. This is a bug.",
                    CHILD1_VIEW_ID,
                    CHILD2_VIEW_ID,
                    view_id
                );
                MessageResult::Stale
            }
        }
    }
}
