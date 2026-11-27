// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use kurbo::{Axis, Insets, Point, Rect, Size};
use tracing::{Span, trace_span};
use understory_anchor::{
    Anchor, AnchorConstraint, AnchorInput, AnchorPolicy, AnchorPositionOption, HysteresisPolicy,
    PreviousAnchorFrame, ScoringPolicy, resolve_anchor,
};

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, SubwindowPlacement, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::imaging::Painter;
use crate::layout::{LenReq, Length, SizeDef};

/// A widget representing the top-level stack of visible subwindows owned by [`RenderRoot`](crate::app::RenderRoot).
///
/// This stack must always have at least one child, representing the "main subwindow",
/// e.g. most of the stuff drawn in the app.
///
/// Other subwindows can represent tooltips, menus, dialogs, etc.
/// They have an associated position and are drawn on top of the main subwindow.
///
/// Ensure that `SubwindowStack` has [`Dimensions`] set via props to [`Dimensions::MAX`].
/// Max preferred size of `SubwindowStack` means that the question of size
/// will get passed through to its main subwindow, and doesn't mean that it will
/// necessarily map to the max preferred size of the main subwindow.
///
/// [`Dimensions`]: crate::properties::Dimensions
/// [`Dimensions::MAX`]: crate::properties::Dimensions::MAX
pub(crate) struct SubwindowStack {
    subwindows: Vec<SubwindowEntry>,
}

/// A persistent entry in the top-level subwindow stack.
///
/// This is widget ownership state, not a derived visual/render subwindow.
struct SubwindowEntry {
    /// Root widget owned by this top-level subwindow entry.
    root: WidgetPod<dyn Widget>,
    /// Placement in the `SubwindowStack`'s content-box coordinates.
    placement: SubwindowPlacement,
    /// Last visible placement for this subwindow's anchor.
    previous_anchor: Option<PreviousAnchorFrame>,
}

// --- MARK: IMPL SUBWINDOW_STACK
impl SubwindowStack {
    /// Creates a stack with the provided main subwindow.
    pub(crate) fn new(root: NewWidget<impl Widget + ?Sized>) -> Self {
        let subwindow = SubwindowEntry {
            root: root.erased().to_pod(),
            placement: SubwindowPlacement::Fixed(Point::ORIGIN),
            previous_anchor: None,
        };
        Self {
            subwindows: vec![subwindow],
        }
    }

    /// Returns the number of subwindows, including the main subwindow.
    pub(crate) fn subwindow_count(&self) -> usize {
        self.subwindows.len()
    }

    /// The [`WidgetId`] of the root widget of the given subwindow.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub(crate) fn subwindow_root_id(&self, idx: usize) -> WidgetId {
        self.subwindows[idx].root.id()
    }
}

// --- MARK: IMPL WIDGETMUT
impl SubwindowStack {
    /// Adds a new subwindow at the end of the stack with the given widget as its root.
    ///
    /// The placement must be in this `SubwindowStack`'s content-box coordinate space.
    /// If this `SubwindowStack` is used as the root widget with no borders, padding, or transforms,
    /// then that coordinate space will exactly match the window's coordinate space.
    pub(crate) fn add_subwindow(
        this: &mut WidgetMut<'_, Self>,
        root: NewWidget<impl Widget + ?Sized>,
        placement: SubwindowPlacement,
    ) {
        let subwindow = SubwindowEntry {
            root: root.erased().to_pod(),
            placement,
            previous_anchor: None,
        };
        this.widget.subwindows.push(subwindow);
        this.ctx.children_changed();
    }

    /// Returns a mutable reference to the root widget of the subwindow at `idx`.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    #[expect(dead_code, reason = "Might be useful later")]
    pub(crate) fn subwindow_root_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> WidgetMut<'t, dyn Widget> {
        let subwindow = &mut this.widget.subwindows[idx].root;
        this.ctx.get_mut(subwindow)
    }

    /// Removes the subwindow with the given widget as root.
    ///
    /// The main subwindow cannot be removed.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the intended subwindow is the main subwindow or
    /// is not found.
    pub(crate) fn remove_subwindow(this: &mut WidgetMut<'_, Self>, root_id: WidgetId) {
        match this
            .widget
            .subwindows
            .iter()
            .position(|subwindow| subwindow.root.id() == root_id)
        {
            Some(0) => debug_panic!("Cannot remove main subwindow"),
            None => debug_panic!("subwindow with root widget {root_id:?} not found"),
            Some(idx) => {
                let child = this.widget.subwindows.remove(idx).root;
                this.ctx.remove_child(child);
            }
        }
    }

    /// Repositions the subwindow with the given widget as root.
    ///
    /// The given `new_origin` must be in this `SubwindowStack`'s content-box coordinate space.
    /// If this `SubwindowStack` is used as the root widget with no borders, padding, or transforms,
    /// then that coordinate space will exactly match the window's coordinate space.
    ///
    /// The main subwindow cannot be repositioned.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the intended subwindow is the main subwindow or
    /// is not found.
    pub(crate) fn reposition_subwindow(
        this: &mut WidgetMut<'_, Self>,
        root_id: WidgetId,
        new_origin: Point,
    ) {
        match this
            .widget
            .subwindows
            .iter()
            .position(|subwindow| subwindow.root.id() == root_id)
        {
            Some(0) => debug_panic!("Cannot reposition main subwindow"),
            None => debug_panic!("subwindow with root widget {root_id:?} not found"),
            Some(idx) => {
                let subwindow = &mut this.widget.subwindows[idx];
                subwindow.placement = SubwindowPlacement::Fixed(new_origin);
                subwindow.previous_anchor = None;
                this.ctx.request_layout();
            }
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for SubwindowStack {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for subwindow in self.subwindows.iter_mut() {
            ctx.register_child(&mut subwindow.root);
        }
    }

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        // First child is the main subwindow, for which we do measure passthrough.
        let Some(main_subwindow) = self.subwindows.first_mut() else {
            debug_panic!("Missing first subwindow");
            return Length::ZERO;
        };
        // Let the main subwindow handle the response
        ctx.redirect_measurement(&mut main_subwindow.root, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // First child is the main subwindow, for which we do layout passthrough.
        let Some(main_subwindow) = self.subwindows.first_mut() else {
            debug_panic!("Missing first subwindow");
            return;
        };

        // Our measurement is just the passed on result from the main subwindow,
        // so the size we received is effectively meant for the main subwindow.
        ctx.run_layout(&mut main_subwindow.root, size);
        // The main subwindow is always located at our origin.
        ctx.place_child(&mut main_subwindow.root, Point::ORIGIN);

        ctx.derive_baselines(&main_subwindow.root);

        let viewport = size.to_rect();
        for subwindow in &mut self.subwindows[1..] {
            // Measure overlays against the window size, using max-content as the fallback.
            let subwindow_size = ctx.compute_size(&mut subwindow.root, SizeDef::MAX, size.into());
            ctx.run_layout(&mut subwindow.root, subwindow_size);
            let origin = subwindow_origin(subwindow, subwindow_size, viewport);
            ctx.place_child(&mut subwindow.root, origin);
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
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
        self.subwindows
            .iter()
            .map(|child| child.root.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("SubwindowStack", id = id.trace())
    }
}

fn subwindow_origin(subwindow: &mut SubwindowEntry, floating_size: Size, viewport: Rect) -> Point {
    let (anchor, preferred, gap) = match subwindow.placement {
        SubwindowPlacement::Fixed(origin) => return origin,
        SubwindowPlacement::Anchored {
            anchor,
            preferred,
            gap,
        } => (anchor, preferred, gap),
    };
    let constraints = [
        AnchorConstraint::Offset {
            main_axis: gap,
            cross_axis: 0.0,
        },
        AnchorConstraint::Shift {
            padding: Insets::ZERO,
        },
    ];
    let option = |placement| AnchorPositionOption::new(placement).with_constraints(&constraints);
    let fallbacks = [option(preferred.opposite())];
    let policy = AnchorPolicy {
        scoring: ScoringPolicy {
            // Avoid rewarding a candidate for shifting closer to the anchor.
            anchor_distance_weight: 0.0,
            ..ScoringPolicy::collision_first()
        },
        hysteresis: HysteresisPolicy {
            enabled: true,
            incumbent_bonus: 0.0,
            // With equal visibility, switch as soon as the other side saves 2 logical pixels of shift.
            switch_threshold: 1.0,
        },
        ..AnchorPolicy::new(option(preferred), &fallbacks)
    };

    let frame = resolve_anchor(
        AnchorInput {
            anchor: Anchor::Rect(anchor),
            floating_size,
            viewport,
            boundary: viewport,
            previous: subwindow.previous_anchor.as_ref(),
        },
        policy,
    );
    subwindow.previous_anchor = frame.visible.then(|| PreviousAnchorFrame::from(&frame));
    frame.rect.origin()
}
