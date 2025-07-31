// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::Span;
use vello::kurbo::{Affine, Insets, Point, Rect, Size, Vec2};

use crate::core::{LayoutCache, WidgetId, WidgetOptions};

// TODO - Reduce WidgetState size.
// See https://github.com/linebender/xilem/issues/706

/// Generic state for all widgets in the hierarchy.
///
/// This struct contains the metadata that passes need to know about widgets and that
/// widgets don't store themselves.
///
/// All context types include a reference to a `WidgetState`, so widgets can query
/// information about their own state and set invalidation flags.
/// Context types for render passes have a shared `&WidgetState`. Context types for
/// other passes have a `&mut WidgetState`.
///
/// In general, the only way to get a `&mut WidgetState` should be in a pass or as part of a
/// [`WidgetMut`]. Both should reborrow the parent widget state and call
/// [`WidgetState::merge_up`] at the end so that invalidations are always bubbled up.
///
/// ## Naming scheme
///
/// Some fields follow a naming scheme:
/// - `request_xxx`: this specific widget has requested the xxx pass to run on it.
/// - `needs_xxx`: this widget or a descendant has requested the xxx pass to run on it.
/// - `is_xxx`: this widget has the xxx property.
/// - `has_xxx`: this widget or a descendant has the xxx property.
///
/// ## Resetting flags
///
/// Generally, the `needs_foobar` and `request_foobar` flags will be reset to
/// false during the "foobar" pass after calling the "foobar" method.
///
/// In principle this shouldn't be a problem because most passes shouldn't be
/// able to request themselves. The exception is the anim pass: an anim frame can
/// (and usually will) request a new anim frame.
///
/// ## Zombie flags
///
/// Masonry's passes should be designed to avoid what we'll call "zombie flags".
///
/// Zombie flags are when a pass "foobar" fails to clear `needs_foobar` flags of some
/// widgets by the time it's complete. Even if the pass works correctly, failing to
/// clear the flags means they'll be propagated up in [`WidgetState::merge_up`] by every
/// other pass, which means the widget tree will keep requesting the same passes over
/// and over. Zombie flags are terrible for performance and power-efficiency.
///
/// For example, in previous versions, stashing widgets could sometimes produce zombie
/// flags, because passes such as paint or layout would not run on stashed widgets, but
/// the `needs_paint` and `needs_layout` flags would still propagate up to the parent.
///
/// To avoid zombie flags, all passes should *always* recurse over all children and *never*
/// exit before recursing. The only short-circuit should be the `if !needs_foobar { return }`
/// block at the beginning of the pass. Flags can be used to skip work in the middle of the
/// pass function, but never to skip the `recurse_on_children` call.
///
/// (The exception is the layout pass, which can't recurse on stashed children.)
///
/// [`WidgetMut`]: crate::core::WidgetMut
#[derive(Clone, Debug)]
pub(crate) struct WidgetState {
    pub(crate) id: WidgetId,

    // --- LAYOUT ---
    /// The size of the widget; this is the value returned by the widget's layout
    /// method.
    pub(crate) size: Size,
    /// The origin of the widget in the `window_transform` coordinate space; together with
    /// `size` these constitute the widget's layout rect.
    pub(crate) origin: Point,
    /// The insets applied to the layout rect to generate the paint rect.
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    pub(crate) paint_insets: Insets,
    // TODO - Document
    // The computed paint rect, in local coordinates.
    pub(crate) local_paint_rect: Rect,
    /// An axis aligned bounding rect (AABB in 2D), containing itself and all its descendents in window coordinates. Includes `paint_insets`.
    pub(crate) bounding_rect: Rect,
    /// The offset of the baseline relative to the bottom of the widget.
    ///
    /// In general, this will be zero; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) baseline_offset: f64,
    /// Data cached from previous layout passes.
    pub(crate) layout_cache: LayoutCache,

    /// Tracks whether widget gets pointer events.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_pointer_interaction: bool,
    /// Tracks whether widget gets text focus.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_focus: bool,

    /// Tracks whether widget is eligible for IME events.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_text_input: bool,
    /// The area of the widget that is being edited by
    /// an IME, in local coordinates.
    pub(crate) ime_area: Option<Rect>,

    // TODO - Use general Shape
    // Currently Kurbo doesn't really provide a type that lets us
    // efficiently hold an arbitrary shape.
    pub(crate) clip_path: Option<Rect>,

    /// Local transform of this widget in the parent coordinate space.
    pub(crate) transform: Affine,
    /// Global transform of this widget in the window coordinate space.
    ///
    /// Computed from all `transform` and `scroll_translation` values from this to the root widget.
    pub(crate) window_transform: Affine,
    /// Translation applied by scrolling, applied after applying `transform` to this widget.
    pub(crate) scroll_translation: Vec2,
    /// The `transform` or `scroll_translation` has changed.
    pub(crate) transform_changed: bool,

    // --- PASSES ---
    /// `WidgetAdded` hasn't been sent to this widget yet.
    pub(crate) is_new: bool,

    /// A flag used to track and debug missing calls to `place_child`.
    pub(crate) is_expecting_place_child_call: bool,

    /// This widget explicitly requested layout
    pub(crate) request_layout: bool,
    /// This widget or a descendant explicitly requested layout
    pub(crate) needs_layout: bool,

    /// The `compose` method must be called on this widget
    pub(crate) request_compose: bool,
    /// The `compose` method must be called on this widget or a descendant
    pub(crate) needs_compose: bool,

    /// The `paint` method must be called on this widget
    pub(crate) request_paint: bool,
    /// The `post_paint` method must be called on this widget
    pub(crate) request_post_paint: bool,
    /// A painting method must be called on this widget or a descendant
    pub(crate) needs_paint: bool,

    /// The `accessibility` method must be called on this widget
    pub(crate) request_accessibility: bool,
    /// The `accessibility` method must be called on this widget or a descendant
    pub(crate) needs_accessibility: bool,

    /// An animation must run on this widget
    pub(crate) request_anim: bool,
    /// An animation must run on this widget or a descendant
    pub(crate) needs_anim: bool,

    /// This widget or a descendant changed its `is_explicitly_disabled` value
    pub(crate) needs_update_disabled: bool,
    /// This widget or a descendant changed its `is_explicitly_stashed` value
    pub(crate) needs_update_stashed: bool,

    pub(crate) needs_update_focus_chain: bool,

    pub(crate) focus_chain: Vec<WidgetId>,

    pub(crate) children_changed: bool,

    // --- STATUS ---
    /// This widget has been disabled.
    pub(crate) is_explicitly_disabled: bool,
    /// This widget or an ancestor has been disabled.
    pub(crate) is_disabled: bool,

    /// This widget has been stashed.
    pub(crate) is_explicitly_stashed: bool,
    /// This widget or an ancestor has been stashed.
    pub(crate) is_stashed: bool,

    /// In the hovered path, starting from window and ending at the hovered widget.
    /// Descendants of the hovered widget are not in the hovered path.
    pub(crate) has_hovered: bool,
    /// This specific widget is hovered.
    pub(crate) is_hovered: bool,

    /// In the active path, starting from window and ending at the active widget.
    /// Descendants of the active widget are not in the active path.
    pub(crate) has_active: bool,
    /// This specific widget is active.
    pub(crate) is_active: bool,

    /// In the focused path, starting from window and ending at the focused widget.
    /// Descendants of the focused widget are not in the focused path.
    pub(crate) has_focus_target: bool,

    // --- DEBUG INFO ---
    /// The typename of the associated widget.
    ///
    /// Used in some guard rails to provide richer error messages when a parent forgets
    /// to iterate over some children.
    pub(crate) trace_span: Span,
    #[cfg(debug_assertions)]
    pub(crate) widget_name: &'static str,
}

impl WidgetState {
    pub(crate) fn new(id: WidgetId, widget_name: &'static str, options: WidgetOptions) -> Self {
        Self {
            id,
            origin: Point::ORIGIN,
            size: Size::ZERO,
            is_expecting_place_child_call: false,
            paint_insets: Insets::ZERO,
            local_paint_rect: Rect::ZERO,
            layout_cache: LayoutCache::empty(),
            accepts_pointer_interaction: true,
            accepts_focus: false,
            accepts_text_input: false,
            ime_area: None,
            clip_path: Option::default(),
            scroll_translation: Vec2::ZERO,
            transform_changed: false,
            is_explicitly_disabled: options.disabled,
            is_explicitly_stashed: false,
            is_disabled: false,
            is_stashed: false,
            baseline_offset: 0.0,
            is_new: true,
            has_hovered: false,
            is_hovered: false,
            has_active: false,
            is_active: false,
            request_layout: true,
            needs_layout: true,
            request_compose: true,
            needs_compose: true,
            request_paint: true,
            request_post_paint: true,
            needs_paint: true,
            request_accessibility: true,
            needs_accessibility: true,
            has_focus_target: false,
            request_anim: true,
            needs_anim: true,
            needs_update_disabled: true,
            needs_update_stashed: true,
            focus_chain: Vec::new(),
            children_changed: true,
            needs_update_focus_chain: true,
            #[cfg(debug_assertions)]
            widget_name,
            window_transform: Affine::IDENTITY,
            bounding_rect: Rect::ZERO,
            trace_span: Span::none(),
            transform: options.transform,
        }
    }

    /// Update to incorporate state changes from a child.
    ///
    /// This method is idempotent and can be called multiple times.
    //
    // TODO: though this method takes child state mutably, child state currently isn't actually
    // mutated anymore. This method may start doing so again in the future, so keep taking &mut for
    // now.
    pub(crate) fn merge_up(&mut self, child_state: &mut Self) {
        self.needs_layout |= child_state.needs_layout;
        self.needs_compose |= child_state.needs_compose;
        self.needs_paint |= child_state.needs_paint;
        self.needs_anim |= child_state.needs_anim;
        self.needs_accessibility |= child_state.needs_accessibility;
        self.needs_update_disabled |= child_state.needs_update_disabled;
        self.children_changed |= child_state.children_changed;
        self.needs_update_focus_chain |= child_state.needs_update_focus_chain;
        self.needs_update_stashed |= child_state.needs_update_stashed;
    }

    /// The paint region for this widget.
    pub(crate) fn paint_rect(&self) -> Rect {
        self.local_paint_rect + self.origin.to_vec2()
    }

    // TODO - Remove
    /// The rectangle used when calculating layout with other widgets.
    pub(crate) fn layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.origin, self.size)
    }

    /// The axis aligned bounding rect of this widget in window coordinates. Includes `paint_insets`.
    ///
    /// This might not map to a visible area of the screen, eg if the widget is scrolled
    /// away.
    pub(crate) fn bounding_rect(&self) -> Rect {
        self.bounding_rect
    }

    /// Returns the area being edited by an IME, in global coordinates.
    ///
    /// By default, returns the same as [`Self::bounding_rect`].
    pub(crate) fn get_ime_area(&self) -> Rect {
        // Note: this returns sensible values for a widget that is translated and/or rescaled.
        // Other transformations like rotation may produce weird IME areas.
        self.window_transform
            .transform_rect_bbox(self.ime_area.unwrap_or_else(|| self.size.to_rect()))
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.window_transform.translation().to_point()
    }

    /// Return the result of intersecting the widget's clip path (if any) with the given rect.
    ///
    /// Both the argument and the result are in window coordinates.
    ///
    /// Returns `None` if the given rect is clipped out.
    pub(crate) fn clip_child(&self, child_rect: Rect) -> Option<Rect> {
        if let Some(clip_path) = self.clip_path {
            let clip_path_global = self.window_transform.transform_rect_bbox(clip_path);
            if clip_path_global.overlaps(child_rect) {
                Some(clip_path_global.intersect(child_rect))
            } else {
                None
            }
        } else {
            Some(child_rect)
        }
    }

    pub(crate) fn needs_rewrite_passes(&self) -> bool {
        self.needs_layout
            || self.needs_compose
            || self.needs_update_disabled
            || self.needs_update_stashed
    }
}
