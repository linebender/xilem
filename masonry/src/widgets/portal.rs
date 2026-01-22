// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use accesskit::{Node, Role};
use dpi::PhysicalPosition;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, ComposeCtx, EventCtx, FromDynWidget, LayoutCtx,
    MeasureCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PointerScrollEvent, PropertiesMut,
    PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::kurbo::{Axis, Point, Rect, Size, Vec2};
use crate::layout::{LayoutSize, LenDef, LenReq, SizeDef};
use crate::widgets::ScrollBar;

// TODO - refactor - see https://github.com/linebender/xilem/issues/366
// TODO - rename "Portal" to "ScrollPortal"?
// TODO - Document which cases need request_layout, request_compose and request_render
// Conceptually, a Portal is a widget giving a restricted view of a child widget
// Imagine a very large widget, and a rect that represents the part of the widget we see

/// A scrolling container with scrollbars and a child widget.
///
/// ## Keyboard and accessibility
///
/// - Exposes an accessibility node with [`accesskit::Role::ScrollView`], including `scroll_x/y`
///   and their ranges.
/// - Handles `accesskit` scroll actions (`ScrollUp`/`ScrollDown`/`ScrollLeft`/`ScrollRight`) by
///   scrolling the viewport.
/// - When this widget is focused, it handles basic keyboard scrolling (arrow keys, PageUp/Down,
///   Home/End) *if the event wasn't already handled by a child*.
///
/// When nested inside another scrolling container, child scroll widgets should call
/// [`EventCtx::set_handled`](crate::core::EventCtx::set_handled) after scrolling to prevent
/// accidental double-scrolling due to event bubbling.
pub struct Portal<W: Widget + ?Sized> {
    child: WidgetPod<W>,
    content_size: Size,
    // TODO - differentiate between the "explicit" viewport pos determined
    // by user input, and the computed viewport pos that may change based
    // on re-layouts
    // TODO - rename
    viewport_pos: Point,
    // TODO - test how it looks like
    constrain_horizontal: bool,
    constrain_vertical: bool,
    must_fill: bool,
    scrollbar_horizontal: WidgetPod<ScrollBar>,
    scrollbar_horizontal_visible: bool,
    scrollbar_vertical: WidgetPod<ScrollBar>,
    scrollbar_vertical_visible: bool,
}

// --- MARK: BUILDERS
impl<W: Widget + ?Sized> Portal<W> {
    /// Creates a scrolling container the given child widget.
    pub fn new(child: NewWidget<W>) -> Self {
        Self {
            child: child.to_pod(),
            content_size: Size::ZERO,
            viewport_pos: Point::ORIGIN,
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
            // TODO - remove (TODO: why?)
            scrollbar_horizontal: WidgetPod::new(ScrollBar::new(Axis::Horizontal, 0.0, 0.0)),
            scrollbar_horizontal_visible: false,
            scrollbar_vertical: WidgetPod::new(ScrollBar::new(Axis::Vertical, 0.0, 0.0)),
            scrollbar_vertical_visible: false,
        }
    }

    /// Builder-style method for constraining the child vertically.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a [`Portal`] lays out its child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its height. The child can be as tall as it wants,
    ///   and the viewport gets moved around to see all of it.
    /// - When it is `true`, the [`Portal`]'s height will be passed down as an upper bound
    ///   on the height of the child. There will be no vertical scrollbar and
    ///   the mouse wheel can't be used to vertically scroll either.
    pub fn constrain_vertical(mut self, constrain: bool) -> Self {
        self.constrain_vertical = constrain;
        self
    }

    /// Builder-style method for constraining the child horizontally.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a [`Portal`] lays out its child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its width. The child can be as wide as it wants,
    ///   and the viewport gets moved around to see all of it.
    /// - When it is `true`, the [`Portal`]'s width will be passed down as an upper bound
    ///   on the width of the child. There will be no horizontal scrollbar and
    ///   the mouse wheel can't be used to horizontally scroll either.
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Builder-style method to set whether the child must fill the view.
    ///
    /// If `true`, the child size is guaranteed to be at least the size of the portal.
    pub fn content_must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
        self
    }
}

pub(crate) fn compute_pan_range(mut viewport: Range<f64>, target: Range<f64>) -> Range<f64> {
    // if either range contains the other, the viewport doesn't move
    if target.start <= viewport.start && viewport.end <= target.end {
        return viewport;
    }
    if viewport.start <= target.start && target.end <= viewport.end {
        return viewport;
    }

    // we compute the length that we need to "fit" in our viewport
    let target_width = f64::min(viewport.end - viewport.start, target.end - target.start);
    let viewport_width = viewport.end - viewport.start;

    // Because of the early returns, there are only two cases to consider: we need
    // to move the viewport "left" or "right"
    if viewport.start >= target.start {
        viewport.start = target.end - target_width;
        viewport.end = viewport.start + viewport_width;
    } else {
        viewport.end = target.start + target_width;
        viewport.start = viewport.end - viewport_width;
    }

    viewport
}

// --- MARK: METHODS
impl<W: Widget + ?Sized> Portal<W> {
    fn update_scrollbars_from_viewport(
        &mut self,
        ctx: &mut EventCtx<'_>,
        portal_size: Size,
        content_size: Size,
    ) {
        let scroll_range = (content_size - portal_size).max(Size::ZERO);

        let progress_x = if scroll_range.width > 1e-12 {
            (self.viewport_pos.x / scroll_range.width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let progress_y = if scroll_range.height > 1e-12 {
            (self.viewport_pos.y / scroll_range.height).clamp(0.0, 1.0)
        } else {
            0.0
        };

        {
            let (scrollbar, mut scrollbar_ctx) = ctx.get_raw_mut(&mut self.scrollbar_horizontal);
            scrollbar.cursor_progress = progress_x;
            scrollbar_ctx.request_render();
            scrollbar_ctx.request_accessibility_update();
        }
        {
            let (scrollbar, mut scrollbar_ctx) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
            scrollbar.cursor_progress = progress_y;
            scrollbar_ctx.request_render();
            scrollbar_ctx.request_accessibility_update();
        }
    }

    fn set_viewport_pos_event_ctx(
        &mut self,
        ctx: &mut EventCtx<'_>,
        portal_size: Size,
        content_size: Size,
        pos: Point,
    ) -> bool {
        let changed = self.set_viewport_pos_raw(portal_size, content_size, pos);
        if changed {
            ctx.request_compose();
            ctx.request_accessibility_update();
            self.update_scrollbars_from_viewport(ctx, portal_size, content_size);
        }
        changed
    }

    fn pan_viewport_by_event_ctx(
        &mut self,
        ctx: &mut EventCtx<'_>,
        portal_size: Size,
        content_size: Size,
        mut delta: Vec2,
    ) -> bool {
        if self.constrain_horizontal {
            delta.x = 0.0;
        }
        if self.constrain_vertical {
            delta.y = 0.0;
        }
        if delta.x == 0.0 && delta.y == 0.0 {
            return false;
        }
        self.set_viewport_pos_event_ctx(ctx, portal_size, content_size, self.viewport_pos + delta)
    }

    fn sync_viewport_from_scrollbars(
        &mut self,
        ctx: &mut EventCtx<'_>,
        portal_size: Size,
        content_size: Size,
    ) -> bool {
        let mut changed = false;
        let scroll_range = (content_size - portal_size).max(Size::ZERO);

        {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_horizontal);
            if scrollbar.moved {
                scrollbar.moved = false;
                let x = scrollbar.cursor_progress * scroll_range.width;
                changed |= self.set_viewport_pos_raw(
                    portal_size,
                    content_size,
                    Point::new(x, self.viewport_pos.y),
                );
            }
        }
        {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
            if scrollbar.moved {
                scrollbar.moved = false;
                let y = scrollbar.cursor_progress * scroll_range.height;
                changed |= self.set_viewport_pos_raw(
                    portal_size,
                    content_size,
                    Point::new(self.viewport_pos.x, y),
                );
            }
        }

        if changed {
            ctx.request_compose();
            ctx.request_accessibility_update();
            self.update_scrollbars_from_viewport(ctx, portal_size, content_size);
        }

        changed
    }

    /// Returns the scrolling "position" of the container.
    pub fn get_viewport_pos(&self) -> Point {
        self.viewport_pos
    }

    // TODO - rename
    fn set_viewport_pos_raw(&mut self, portal_size: Size, content_size: Size, pos: Point) -> bool {
        let viewport_max_pos = (content_size - portal_size).max(Size::ZERO);
        let pos = Point::new(
            pos.x.clamp(0.0, viewport_max_pos.width),
            pos.y.clamp(0.0, viewport_max_pos.height),
        );

        if (pos - self.viewport_pos).hypot2() > 1e-12 {
            self.viewport_pos = pos;
            true
        } else {
            false
        }
    }

    // Note - Rect is in child coordinates
    // TODO - Merge with pan_viewport_to
    // Right now these functions are just different enough to be a pain to merge.
    fn pan_viewport_to_raw(&mut self, portal_size: Size, content_size: Size, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(self.viewport_pos, portal_size);

        let new_pos_x = compute_pan_range(
            viewport.min_x()..viewport.max_x(),
            target.min_x()..target.max_x(),
        )
        .start;
        let new_pos_y = compute_pan_range(
            viewport.min_y()..viewport.max_y(),
            target.min_y()..target.max_y(),
        )
        .start;

        self.set_viewport_pos_raw(portal_size, content_size, Point::new(new_pos_x, new_pos_y))
    }
}

// --- MARK: WIDGETMUT
impl<W: Widget + FromDynWidget + ?Sized> Portal<W> {
    /// Replaces the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<W>) {
        this.ctx
            .remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
    }

    /// Returns mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        this.ctx.get_mut(&mut this.widget.child)
    }

    /// Returns mutable reference to the horizontal scrollbar.
    pub fn horizontal_scrollbar_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, ScrollBar> {
        this.ctx.get_mut(&mut this.widget.scrollbar_horizontal)
    }

    /// Returns mutable reference to the vertical scrollbar.
    pub fn vertical_scrollbar_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, ScrollBar> {
        this.ctx.get_mut(&mut this.widget.scrollbar_vertical)
    }

    /// Sets whether to constrain the child horizontally.
    ///
    /// See [`Portal::constrain_horizontal`] for more details.
    pub fn set_constrain_horizontal(this: &mut WidgetMut<'_, Self>, constrain: bool) {
        this.widget.constrain_horizontal = constrain;
        this.ctx.request_layout();
    }

    /// Sets whether to constrain the child vertically.
    ///
    /// See [`Portal::constrain_vertical`] for more details.
    pub fn set_constrain_vertical(this: &mut WidgetMut<'_, Self>, constrain: bool) {
        this.widget.constrain_vertical = constrain;
        this.ctx.request_layout();
    }

    /// Sets whether the child's size must be greater than or equal the size of
    /// the `Portal`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: Portal::content_must_fill
    pub fn set_content_must_fill(this: &mut WidgetMut<'_, Self>, must_fill: bool) {
        this.widget.must_fill = must_fill;
        this.ctx.request_layout();
    }

    /// Sets the scrolling "position" of the container.
    ///
    /// A position of zero means no scrolling at all.
    pub fn set_viewport_pos(this: &mut WidgetMut<'_, Self>, position: Point) -> bool {
        let portal_size = this.ctx.size();
        let content_size = this.ctx.get_mut(&mut this.widget.child).ctx.size();

        let pos_changed = this
            .widget
            .set_viewport_pos_raw(portal_size, content_size, position);
        if pos_changed {
            let progress_x = this.widget.viewport_pos.x / (content_size - portal_size).width;
            Self::horizontal_scrollbar_mut(this).widget.cursor_progress = progress_x;
            Self::horizontal_scrollbar_mut(this).ctx.request_render();
            let progress_y = this.widget.viewport_pos.y / (content_size - portal_size).height;
            Self::vertical_scrollbar_mut(this).widget.cursor_progress = progress_y;
            Self::vertical_scrollbar_mut(this).ctx.request_render();
            this.ctx.request_layout();
        }
        pos_changed
    }

    /// Translates the scrolling "position" of the container.
    pub fn pan_viewport_by(this: &mut WidgetMut<'_, Self>, translation: Vec2) -> bool {
        Self::set_viewport_pos(this, this.widget.viewport_pos + translation)
    }

    /// Changes the scrolling "position" of the container so that `target` is scrolled into view.
    ///
    /// `target` is in child coordinates, meaning a target of `(0, 0, 10, 10)` will
    /// scroll an item at the top-left of the child into view.
    pub fn pan_viewport_to(this: &mut WidgetMut<'_, Self>, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(this.widget.viewport_pos, this.ctx.size());

        let new_pos_x = compute_pan_range(
            viewport.min_x()..viewport.max_x(),
            target.min_x()..target.max_x(),
        )
        .start;
        let new_pos_y = compute_pan_range(
            viewport.min_y()..viewport.max_y(),
            target.min_y()..target.max_y(),
        )
        .start;

        Self::set_viewport_pos(this, Point::new(new_pos_x, new_pos_y))
    }
}

// --- MARK: IMPL WIDGET
impl<W: Widget + FromDynWidget + ?Sized> Widget for Portal<W> {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let portal_size = ctx.size();
        let content_size = self.content_size;

        match *event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                // TODO - Remove reference to scale factor.
                // See https://github.com/linebender/xilem/issues/1264
                let scale_factor = ctx.get_scale_factor();
                let line_px = PhysicalPosition {
                    x: 120.0 * scale_factor,
                    y: 120.0 * scale_factor,
                };
                let page_px = PhysicalPosition {
                    x: portal_size.width * scale_factor,
                    y: portal_size.height * scale_factor,
                };
                let delta_px = delta.to_pixel_delta(line_px, page_px);
                let dpi::LogicalPosition { x, y } = delta_px.to_logical::<f64>(scale_factor);
                let mut delta = -Vec2 { x, y };

                // Ignore scroll deltas in directions that are constrained
                if self.constrain_horizontal {
                    delta.x = 0.;
                }
                if self.constrain_vertical {
                    delta.y = 0.;
                }

                if self.pan_viewport_by_event_ctx(ctx, portal_size, content_size, delta) {
                    ctx.set_handled();
                };
            }
            _ => (),
        }

        // This section works because events are propagated up. So if the scrollbar got
        // pointer events, then its event method has already been called by the time this runs.
        if self.sync_viewport_from_scrollbars(ctx, portal_size, content_size) {
            ctx.set_handled();
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let portal_size = ctx.size();
        let content_size = self.content_size;
        let target = ctx.target();
        let scrollbar_target =
            target == self.scrollbar_vertical.id() || target == self.scrollbar_horizontal.id();

        if let TextEvent::Keyboard(event) = event
            && event.state.is_down()
            // Avoid scrolling the portal when the focused widget is one of its scrollbars.
            // Scrollbars are focusable for keyboard users, and in that case they should own
            // the arrow/page/home/end keys.
            && !scrollbar_target
        {
            // TODO: Scale factor handling is in flux; revisit as part of
            // https://github.com/linebender/xilem/issues/1264.
            let scale = ctx.get_scale_factor();

            let line = 120.0 * scale;
            let page_y = portal_size.height * scale;

            use crate::core::keyboard::{Key, NamedKey};
            let mut did_scroll = false;
            match &event.key {
                Key::Named(NamedKey::PageDown) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(0.0, page_y),
                    );
                }
                Key::Named(NamedKey::PageUp) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(0.0, -page_y),
                    );
                }
                Key::Named(NamedKey::ArrowDown) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(0.0, line),
                    );
                }
                Key::Named(NamedKey::ArrowUp) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(0.0, -line),
                    );
                }
                Key::Named(NamedKey::ArrowRight) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(line, 0.0),
                    );
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    did_scroll |= self.pan_viewport_by_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Vec2::new(-line, 0.0),
                    );
                }
                Key::Named(NamedKey::Home) => {
                    did_scroll |= self.set_viewport_pos_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Point::new(0.0, 0.0),
                    );
                }
                Key::Named(NamedKey::End) => {
                    let scroll_range = (content_size - portal_size).max(Size::ZERO);
                    did_scroll |= self.set_viewport_pos_event_ctx(
                        ctx,
                        portal_size,
                        content_size,
                        Point::new(scroll_range.width, scroll_range.height),
                    );
                }
                _ => {}
            }
            if did_scroll {
                ctx.set_handled();
            }
        }

        // Events bubble; if a scrollbar handled the keypress and updated its cursor progress,
        // we synchronize the portal viewport here.
        if self.sync_viewport_from_scrollbars(ctx, portal_size, content_size) {
            ctx.set_handled();
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        let portal_size = ctx.size();
        let content_size = self.content_size;
        let target = ctx.target();
        let scrollbar_target =
            target == self.scrollbar_vertical.id() || target == self.scrollbar_horizontal.id();

        if !scrollbar_target
            && matches!(
                event.action,
                accesskit::Action::ScrollUp
                    | accesskit::Action::ScrollDown
                    | accesskit::Action::ScrollLeft
                    | accesskit::Action::ScrollRight
            )
        {
            // TODO: Scale factor handling is in flux; revisit as part of
            // https://github.com/linebender/xilem/issues/1264.
            let scale = ctx.get_scale_factor();

            let unit = if let Some(accesskit::ActionData::ScrollUnit(unit)) = &event.data {
                *unit
            } else {
                accesskit::ScrollUnit::Item
            };
            let line = 120.0 * scale;
            let amount = match unit {
                accesskit::ScrollUnit::Item => line,
                accesskit::ScrollUnit::Page => match event.action {
                    accesskit::Action::ScrollLeft | accesskit::Action::ScrollRight => {
                        portal_size.width * scale
                    }
                    _ => portal_size.height * scale,
                },
            };

            let delta = match event.action {
                accesskit::Action::ScrollUp => Vec2::new(0.0, -amount),
                accesskit::Action::ScrollDown => Vec2::new(0.0, amount),
                accesskit::Action::ScrollLeft => Vec2::new(-amount, 0.0),
                accesskit::Action::ScrollRight => Vec2::new(amount, 0.0),
                _ => Vec2::ZERO,
            };

            if self.pan_viewport_by_event_ctx(ctx, portal_size, content_size, delta) {
                ctx.set_handled();
            }
        }

        // Events bubble; if a scrollbar handled the accessibility action and updated its cursor
        // progress, we synchronize the portal viewport here.
        if self.sync_viewport_from_scrollbars(ctx, portal_size, content_size) {
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
        ctx.register_child(&mut self.scrollbar_horizontal);
        ctx.register_child(&mut self.scrollbar_vertical);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::RequestPanToChild(target) => {
                let portal_size = ctx.size();
                let content_size = self.content_size;

                self.pan_viewport_to_raw(portal_size, content_size, *target);
                ctx.request_compose();

                // TODO - There's a lot of code here that's duplicated from the `MouseWheel`
                // event in `on_pointer_event`.
                // Because this code directly manipulates child widgets, it's hard to factor
                // it out.
                let (scrollbar, mut scrollbar_ctx) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
                scrollbar.cursor_progress =
                    self.viewport_pos.y / (content_size - portal_size).height;
                scrollbar_ctx.request_render();

                drop(scrollbar_ctx);

                let (scrollbar, mut scrollbar_ctx) =
                    ctx.get_raw_mut(&mut self.scrollbar_horizontal);
                scrollbar.cursor_progress =
                    self.viewport_pos.x / (content_size - portal_size).width;
                scrollbar_ctx.request_render();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        match len_req {
            LenReq::MinContent => 0.,
            LenReq::MaxContent => {
                let context_size = LayoutSize::maybe(axis.cross(), cross_length);
                let auto_length = len_req.into();

                let cross = axis.cross();
                let cross_space = cross_length.filter(|_| match cross {
                    Axis::Horizontal => self.constrain_horizontal,
                    Axis::Vertical => self.constrain_vertical,
                });

                ctx.compute_length(
                    &mut self.child,
                    auto_length,
                    context_size,
                    axis,
                    cross_space,
                )
            }
            LenReq::FitContent(space) => space,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let auto_size = SizeDef::new(
            match self.constrain_horizontal {
                true => LenDef::FitContent(size.width),
                false => LenDef::MaxContent,
            },
            match self.constrain_vertical {
                true => LenDef::FitContent(size.height),
                false => LenDef::MaxContent,
            },
        );
        let content_size = {
            let child_size = ctx.compute_size(&mut self.child, auto_size, size.into());
            if self.must_fill {
                child_size.max(size)
            } else {
                child_size
            }
        };
        ctx.run_layout(&mut self.child, content_size);
        self.content_size = content_size;

        // TODO - document better
        // Recompute the portal offset for the new layout
        self.set_viewport_pos_raw(size, content_size, self.viewport_pos);
        // TODO - recompute portal progress

        ctx.set_clip_path(size.to_rect());

        ctx.place_child(&mut self.child, Point::ZERO);

        self.scrollbar_horizontal_visible =
            !self.constrain_horizontal && size.width < content_size.width;
        self.scrollbar_vertical_visible =
            !self.constrain_vertical && size.height < content_size.height;

        ctx.set_stashed(
            &mut self.scrollbar_horizontal,
            !self.scrollbar_horizontal_visible,
        );
        if self.scrollbar_horizontal_visible {
            let (scrollbar, mut sb_ctx) = ctx.get_raw_mut(&mut self.scrollbar_horizontal);
            scrollbar.portal_size = size.width;
            scrollbar.content_size = content_size.width;
            sb_ctx.request_render();
            drop(sb_ctx);

            let scrollbar_size = ctx.compute_size(
                &mut self.scrollbar_horizontal,
                SizeDef::fit(size),
                size.into(),
            );
            ctx.run_layout(&mut self.scrollbar_horizontal, scrollbar_size);
            ctx.place_child(
                &mut self.scrollbar_horizontal,
                Point::new(0.0, size.height - scrollbar_size.height),
            );
        }

        ctx.set_stashed(
            &mut self.scrollbar_vertical,
            !self.scrollbar_vertical_visible,
        );
        if self.scrollbar_vertical_visible {
            let (scrollbar, mut sb_ctx) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
            scrollbar.portal_size = size.height;
            scrollbar.content_size = content_size.height;
            sb_ctx.request_render();
            drop(sb_ctx);

            let scrollbar_size = ctx.compute_size(
                &mut self.scrollbar_vertical,
                SizeDef::fit(size),
                size.into(),
            );
            ctx.run_layout(&mut self.scrollbar_vertical, scrollbar_size);
            ctx.place_child(
                &mut self.scrollbar_vertical,
                Point::new(size.width - scrollbar_size.width, 0.0),
            );
        }
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        ctx.set_child_scroll_translation(
            &mut self.child,
            Vec2::new(-self.viewport_pos.x, -self.viewport_pos.y),
        );
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::ScrollView
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_clips_children();

        let portal_size = ctx.size();
        let content_size = self.content_size;
        let scroll_range = (content_size - portal_size).max(Size::ZERO);

        let can_scroll_x = !self.constrain_horizontal && scroll_range.width > 1e-12;
        let can_scroll_y = !self.constrain_vertical && scroll_range.height > 1e-12;

        if can_scroll_x {
            node.set_scroll_x_min(0.0);
            node.set_scroll_x_max(scroll_range.width);
            node.set_scroll_x(self.viewport_pos.x.clamp(0.0, scroll_range.width));
            if self.viewport_pos.x > 1e-12 {
                node.add_action(accesskit::Action::ScrollLeft);
            }
            if self.viewport_pos.x + 1e-12 < scroll_range.width {
                node.add_action(accesskit::Action::ScrollRight);
            }
        } else {
            node.clear_scroll_x_min();
            node.clear_scroll_x_max();
            node.clear_scroll_x();
        }

        if can_scroll_y {
            node.set_scroll_y_min(0.0);
            node.set_scroll_y_max(scroll_range.height);
            node.set_scroll_y(self.viewport_pos.y.clamp(0.0, scroll_range.height));
            if self.viewport_pos.y > 1e-12 {
                node.add_action(accesskit::Action::ScrollUp);
            }
            if self.viewport_pos.y + 1e-12 < scroll_range.height {
                node.add_action(accesskit::Action::ScrollDown);
            }
        } else {
            node.clear_scroll_y_min();
            node.clear_scroll_y_max();
            node.clear_scroll_y();
        }

        if can_scroll_y && !can_scroll_x {
            node.set_orientation(accesskit::Orientation::Vertical);
        } else if can_scroll_x && !can_scroll_y {
            node.set_orientation(accesskit::Orientation::Horizontal);
        } else {
            node.clear_orientation();
        }

        node.add_child_action(accesskit::Action::ScrollIntoView);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[
            self.child.id(),
            self.scrollbar_vertical.id(),
            self.scrollbar_horizontal.id(),
        ])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Portal", id = id.trace())
    }

    fn accepts_focus(&self) -> bool {
        !(self.constrain_horizontal && self.constrain_vertical)
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::WidgetTag;
    use crate::core::keyboard::{Key, NamedKey};
    use crate::layout::AsUnit;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Button, Flex, SizedBox};

    fn button(text: &'static str) -> impl Widget {
        SizedBox::new(Button::with_text(text).with_auto_id())
            .width(70.px())
            .height(40.px())
    }

    #[test]
    fn button_list() {
        let button_3 = WidgetTag::named("button-3");
        let button_13 = WidgetTag::named("button-13");

        let widget = Portal::new(NewWidget::new(
            Flex::column()
                .with_fixed(button("Item 1").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 2").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(NewWidget::new_with_tag(button("Item 3"), button_3))
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 4").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 5").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 6").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 7").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 8").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 9").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 10").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 11").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 12").with_auto_id())
                .with_fixed_spacer(10.px())
                .with_fixed(NewWidget::new_with_tag(button("Item 13"), button_13))
                .with_fixed_spacer(10.px())
                .with_fixed(button("Item 14").with_auto_id())
                .with_fixed_spacer(10.px()),
        ))
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(400., 400.));

        assert_render_snapshot!(harness, "portal_button_list_no_scroll");

        harness.edit_root_widget(|mut portal| {
            Portal::set_viewport_pos(&mut portal, Point::new(0.0, 130.0))
        });

        assert_render_snapshot!(harness, "portal_button_list_scrolled");

        let item_3_rect = harness.get_widget(button_3).ctx().local_layout_rect();
        harness.edit_root_widget(|mut portal| {
            Portal::pan_viewport_to(&mut portal, item_3_rect);
        });

        assert_render_snapshot!(harness, "portal_button_list_scroll_to_item_3");

        let item_13_rect = harness.get_widget(button_13).ctx().local_layout_rect();
        harness.edit_root_widget(|mut portal| {
            Portal::pan_viewport_to(&mut portal, item_13_rect);
        });

        assert_render_snapshot!(harness, "portal_button_list_scroll_to_item_13");
    }

    #[test]
    fn scroll_into_view() {
        let button_tag = WidgetTag::named("hidden-button");

        let widget = Portal::new(
            Flex::column()
                .with_fixed_spacer(500.px())
                .with_fixed(NewWidget::new_with_tag(
                    Button::with_text("Fully visible"),
                    button_tag,
                ))
                .with_fixed_spacer(500.px())
                .with_auto_id(),
        )
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200., 200.));
        let button_id = harness.get_widget(button_tag).id();

        harness.scroll_into_view(button_id);
        assert_render_snapshot!(harness, "portal_scrolled_button_into_view");
    }

    #[test]
    fn portal_accessibility_node_exposes_scroll() {
        let portal_tag = WidgetTag::named("portal");
        let content = SizedBox::empty().size(300.px(), 300.px()).with_auto_id();
        let portal = NewWidget::new_with_tag(Portal::new(content), portal_tag);

        let mut harness =
            TestHarness::create_with_size(test_property_set(), portal, Size::new(100.0, 100.0));
        let _ = harness.render();

        let portal_id = harness.get_widget(portal_tag).id();
        let node = harness.access_node(portal_id).unwrap();

        assert_eq!(node.data().role(), Role::ScrollView);
        assert!(node.data().supports_action(accesskit::Action::ScrollDown));
        assert!(!node.data().supports_action(accesskit::Action::ScrollUp));
        assert!(
            node.data()
                .child_supports_action(accesskit::Action::ScrollIntoView)
        );
        assert_eq!(node.data().scroll_y_min(), Some(0.0));
        assert!(node.data().scroll_y_max().is_some());
        assert_eq!(node.data().scroll_y(), Some(0.0));
    }

    #[test]
    fn portal_keyboard_scroll_updates_access_tree() {
        let portal_tag = WidgetTag::named("portal");
        let content = SizedBox::empty().size(300.px(), 300.px()).with_auto_id();
        let portal = NewWidget::new_with_tag(Portal::new(content), portal_tag);

        let mut harness =
            TestHarness::create_with_size(test_property_set(), portal, Size::new(100.0, 100.0));
        let _ = harness.render();

        let portal_id = harness.get_widget(portal_tag).id();
        harness.focus_on(Some(portal_id));

        harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::PageDown)));
        let _ = harness.render();

        let node = harness.access_node(portal_id).unwrap();
        assert!(node.data().scroll_y().unwrap_or(0.0) > 0.0);
    }

    // Helper function for panning tests
    fn make_range(repr: &str) -> Range<f64> {
        let repr = &repr[repr.find('_').unwrap()..];

        let start = repr.find('x').unwrap();
        let end = repr[start..].find('_').unwrap() + start;

        assert!(repr[end..].chars().all(|c| c == '_'));

        (start as f64)..(end as f64)
    }

    #[test]
    fn test_pan_to_same() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" _______xxxx_____");
        let result_range = make_range(" _______xxxx_____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_smaller() {
        let initial_range = make_range("_____xxxxxxxx___");
        let target_range = make_range(" _______xxxx_____");
        let result_range = make_range(" _____xxxxxxxx___");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_larger() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" _____xxxxxxxx___");
        let result_range = make_range(" _______xxxx_____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" ____xx__________");
        let result_range = make_range(" ____xxxx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_intersects() {
        let initial_range = make_range("_______xxxxx____");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ____xxxxx_______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_larger() {
        let initial_range = make_range("__________xx____");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ______xx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_larger_intersects() {
        let initial_range = make_range("_______xx_______");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ______xx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right() {
        let initial_range = make_range("_____xxxx_______");
        let target_range = make_range(" __________xx____");
        let result_range = make_range(" ________xxxx____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_intersects() {
        let initial_range = make_range("____xxxxx_______");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" _______xxxxx____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_larger() {
        let initial_range = make_range("____xx__________");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" ________xx______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_larger_intersects() {
        let initial_range = make_range("_______xx_______");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" ________xx______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }
}
