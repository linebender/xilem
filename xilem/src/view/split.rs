// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widgets::{self, Axis};
use xilem_core::{DynMessage, MessageResult, View, ViewId, ViewMarker, ViewPathTracker};

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
{
    Split {
        split_axis: Axis::Horizontal,
        split_point: 0.5,
        min_size: (0.0, 0.0),
        bar_size: 6.0,
        min_bar_area: 6.0,
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
    min_size: (f64, f64), // Integers only
    bar_size: f64,        // Integers only
    min_bar_area: f64,    // Integers only
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
    /// The value must be greater than or equal to `0.0`.
    /// The value will be rounded up to the nearest integer.
    pub fn min_size(mut self, first: f64, second: f64) -> Self {
        assert!(first >= 0.0);
        assert!(second >= 0.0);
        self.min_size = (first.ceil(), second.ceil());
        self
    }

    /// Set the size of the splitter bar in logical pixels.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    #[track_caller]
    pub fn bar_size(mut self, bar_size: f64) -> Self {
        assert!(bar_size >= 0.0, "bar_size must be 0.0 or greater!");
        self.bar_size = bar_size.ceil();
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
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    #[track_caller]
    pub fn min_bar_area(mut self, min_bar_area: f64) -> Self {
        assert!(min_bar_area >= 0.0, "min_bar_area must be 0.0 or greater!");
        self.min_bar_area = min_bar_area.ceil();
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
    State: 'static,
    Action: 'static,
    ChildA: WidgetView<State, Action>,
    ChildB: WidgetView<State, Action>,
{
    type Element = Pod<widgets::Split<ChildA::Widget, ChildB::Widget>>;

    type ViewState = (ChildA::ViewState, ChildB::ViewState);

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child1, child1_state) =
            ctx.with_id(CHILD1_VIEW_ID, |ctx| self.child1.build(ctx, app_state));
        let (child2, child2_state) =
            ctx.with_id(CHILD2_VIEW_ID, |ctx| self.child2.build(ctx, app_state));

        let widget_pod = ctx.create_pod(
            widgets::Split::new_pod(child1.into_widget_pod(), child2.into_widget_pod())
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
        mut element: xilem_core::Mut<'_, Self::Element>,
        app_state: &mut State,
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
                app_state,
            );
        });

        ctx.with_id(CHILD2_VIEW_ID, |ctx| {
            let child2_element = widgets::Split::child2_mut(&mut element);
            self.child2.rebuild(
                &prev.child2,
                &mut view_state.1,
                ctx,
                child2_element,
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: xilem_core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let child1_element = widgets::Split::child1_mut(&mut element);
        self.child1
            .teardown(&mut view_state.0, ctx, child1_element, app_state);

        let child2_element = widgets::Split::child2_mut(&mut element);
        self.child2
            .teardown(&mut view_state.1, ctx, child2_element, app_state);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action, DynMessage> {
        match id_path.split_first() {
            Some((&CHILD1_VIEW_ID, rest)) => {
                self.child1
                    .message(&mut view_state.0, rest, message, app_state)
            }
            Some((&CHILD2_VIEW_ID, rest)) => {
                self.child2
                    .message(&mut view_state.1, rest, message, app_state)
            }
            view_id => {
                tracing::error!(
                    "Invalid message arrived in Split::message, expected {:?} or {:?}, got {:?}. This is a bug.",
                    CHILD1_VIEW_ID,
                    CHILD2_VIEW_ID,
                    view_id
                );
                MessageResult::Stale(message)
            }
        }
    }
}
