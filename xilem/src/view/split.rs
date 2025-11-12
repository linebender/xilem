// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::Axis;
use masonry::properties::types::{AsUnit, Length};
use masonry::widgets::{self, ceil_length};

use crate::core::{
    Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewId, ViewMarker, ViewPathTracker,
};
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
    State: ViewArgument,
{
    Split {
        split_axis: Axis::Horizontal,
        split_point: 0.5,
        min_size: (Length::ZERO, Length::ZERO),
        bar_size: 6.px(),
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
    split_point: f64,
    min_size: (Length, Length), // Integers only
    bar_size: Length,           // Integers only
    min_bar_area: Length,       // Integers only
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
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    #[track_caller]
    pub fn split_point(mut self, split_point: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&split_point),
            "split_point must be in the range [0.0, 1.0], got {split_point}"
        );
        self.split_point = split_point;
        self
    }

    /// Set the minimum size for both sides of the split axis in logical pixels.
    ///
    /// The value will be rounded up to the nearest integer.
    pub fn min_size(mut self, first: Length, second: Length) -> Self {
        self.min_size = (ceil_length(first), ceil_length(second));
        self
    }

    /// Set the size of the splitter bar in logical pixels.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    #[track_caller]
    pub fn bar_size(mut self, bar_size: Length) -> Self {
        self.bar_size = ceil_length(bar_size);
        self
    }

    /// Set the minimum size of the splitter bar area in logical pixels.
    ///
    /// The minimum splitter bar area defines the minimum size of the area
    /// where mouse hit detection is done for the splitter bar.
    /// The final area is either this or the splitter bar size, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    #[track_caller]
    pub fn min_bar_area(mut self, min_bar_area: Length) -> Self {
        self.min_bar_area = ceil_length(min_bar_area);
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

const CHILD1_VIEW_ID: ViewId = ViewId::new(0);
const CHILD2_VIEW_ID: ViewId = ViewId::new(1);

impl<ChildA, ChildB, State, Action> ViewMarker for Split<ChildA, ChildB, State, Action> {}
impl<ChildA, ChildB, State, Action> View<State, Action, ViewCtx>
    for Split<ChildA, ChildB, State, Action>
where
    State: ViewArgument,
    Action: 'static,
    ChildA: WidgetView<State, Action>,
    ChildB: WidgetView<State, Action>,
{
    type Element = Pod<widgets::Split<ChildA::Widget, ChildB::Widget>>;

    type ViewState = (ChildA::ViewState, ChildB::ViewState);

    fn build(
        &self,
        ctx: &mut ViewCtx,
        mut app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (child1, child1_state) = ctx.with_id(CHILD1_VIEW_ID, |ctx| {
            self.child1.build(ctx, State::reborrow_mut(&mut app_state))
        });
        let (child2, child2_state) = ctx.with_id(CHILD2_VIEW_ID, |ctx| {
            self.child2.build(ctx, State::reborrow_mut(&mut app_state))
        });

        let widget_pod = ctx.create_pod(
            widgets::Split::new(child1.new_widget, child2.new_widget)
                .split_axis(self.split_axis)
                .split_point(self.split_point)
                .min_size(self.min_size.0, self.min_size.1)
                .bar_size(self.bar_size)
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
        mut app_state: Arg<'_, State>,
    ) {
        if prev.split_axis != self.split_axis {
            widgets::Split::set_split_axis(&mut element, self.split_axis);
        }

        if prev.split_point != self.split_point {
            widgets::Split::set_split_point(&mut element, self.split_point);
        }

        if prev.min_size != self.min_size {
            widgets::Split::set_min_size(&mut element, self.min_size.0, self.min_size.1);
        }

        if prev.bar_size != self.bar_size {
            widgets::Split::set_bar_size(&mut element, self.bar_size);
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
                State::reborrow_mut(&mut app_state),
            );
        });

        ctx.with_id(CHILD2_VIEW_ID, |ctx| {
            let child2_element = widgets::Split::child2_mut(&mut element);
            self.child2.rebuild(
                &prev.child2,
                &mut view_state.1,
                ctx,
                child2_element,
                State::reborrow_mut(&mut app_state),
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
        app_state: Arg<'_, State>,
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
