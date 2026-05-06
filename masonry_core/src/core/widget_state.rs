// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
use tracing::Span;

use crate::core::{
    ClassSetDiff, PaintLayerMode, PropertyCache, PropertyStackId, WidgetId, WidgetOptions,
};
use crate::layout::MeasurementCache;

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
/// # Naming scheme
///
/// Some fields follow a naming scheme:
/// - `request_xxx`: this specific widget has requested the xxx pass to run on it.
/// - `needs_xxx`: this widget or a descendant has requested the xxx pass to run on it.
/// - `is_xxx`: this widget has the xxx property.
/// - `has_xxx`: this widget or a descendant has the xxx property.
///
/// # Resetting flags
///
/// Generally, the `needs_foobar` and `request_foobar` flags will be reset to
/// false during the "foobar" pass after calling the "foobar" method.
///
/// In principle this shouldn't be a problem because most passes shouldn't be
/// able to request themselves. The exception is the anim pass: an anim frame can
/// (and usually will) request a new anim frame.
///
/// # Zombie flags
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
    /// The origin (top-left) of the widget's layout border-box
    /// in the parent's layout border-box coordinate space.
    pub(crate) layout_origin: Point,
    /// The size of the widget's layout border-box.
    ///
    /// This is also used to determine layout cache validity.
    pub(crate) layout_border_box_size: Size,
    /// The insets for converting between content-box and border-box rects.
    ///
    /// Add these insets to the content-box to get the border-box,
    /// and subtract these insets from the border-box to get the content-box.
    ///
    /// These insets are derived from the widget's border and padding properties.
    pub(crate) border_box_insets: Insets,
    /// The insets for converting between border-box and paint-box rects.
    ///
    /// Add these insets to the border-box to get the paint-box,
    /// and subtract these insets from the paint-box to get the border-box.
    ///
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    pub(crate) paint_box_insets: Insets,
    /// The widget's visual border-box in the widget's layout border-box coordinate space.
    ///
    /// This is resolved during compose from the layout border-box. It may be pixel-snapped using
    /// the widget's full transform to the window's coordinate space and the current scale factor.
    /// When snapping is not supported for the transform, then this matches the layout border-box.
    pub(crate) visual_border_box: Rect,
    /// An axis aligned bounding rect (AABB in 2D),
    /// containing itself and all its descendents in the window's coordinate space.
    ///
    /// This is the union of clipped visual paint-box rects in the window's coordinate space.
    pub(crate) bounding_box: Rect,

    /// The offset of the first baseline relative to the top of the widget's layout border-box.
    ///
    /// In general, this will be `f64::NAN`; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) first_baseline: f64,
    /// The offset of the last baseline relative to the top of the widget's layout border-box.
    ///
    /// In general, this will be `f64::NAN`; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) last_baseline: f64,

    // TODO - Use general Shape
    // Currently Kurbo doesn't really provide a type that lets us
    // efficiently hold an arbitrary shape.
    /// The widget's clip path in the widget's layout border-box coordinate space.
    ///
    /// This clips the painting of `Widget::paint` and all the painting of children.
    /// It does not clip this widget's `Widget::pre_paint` nor `Widget::post_paint`.
    pub(crate) clip_path: Option<Rect>,

    /// Local transform used during the mapping of this widget's layout border-box coordinate
    /// space to the parent's layout border-box coordinate space.
    ///
    /// When mapping this widget's visual border-box into the window's coordinate space,
    /// first this transform will be applied and then `scroll_translation` and `origin`
    /// applied on top.
    pub(crate) transform: Affine,
    /// Global transform mapping this widget's layout border-box coordinate space
    /// to the window's coordinate space.
    ///
    /// Computed from all `transform`, `scroll_translation`, and `origin` values
    /// from this widget all the way up to the window.
    ///
    /// Multiply by this to convert from this widget's layout border-box coordinate space to the
    /// window's coordinate space, or use the inverse of this transform to go from window's space
    /// to this widget's layout border-box coordinate space.
    pub(crate) window_transform: Affine,
    /// Translation applied by scrolling, applied after applying `transform` to this widget.
    pub(crate) scroll_translation: Vec2,
    /// The `transform` or `scroll_translation` has changed.
    pub(crate) transform_changed: bool,

    // --- INTERACTIONS ---
    /// The `TypeId` of the widget's `Widget::Action` type.
    pub(crate) action_type: TypeId,

    /// Tracks whether widget gets pointer events.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_pointer_interaction: bool,
    /// Tracks whether children of this widget get pointer events.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) propagates_pointer_interaction: bool,
    /// Tracks whether widget gets text focus.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_focus: bool,

    /// Tracks whether widget is eligible for IME events.
    /// Should be immutable after `WidgetAdded` event.
    pub(crate) accepts_text_input: bool,
    /// The area of the widget that is being edited by an IME,
    /// in the widget's layout border-box coordinate space.
    pub(crate) ime_area: Option<Rect>,

    // --- PASSES ---
    /// `WidgetAdded` hasn't been sent to this widget yet.
    pub(crate) is_new: bool,

    /// A flag used to track and debug missing calls to `place_child`.
    pub(crate) is_expecting_place_child_call: bool,

    /// This widget explicitly requested layout
    pub(crate) request_layout: bool,
    /// This widget or a descendant explicitly requested layout
    needs_layout: bool,
    /// Cached measurement results.
    pub(crate) measurement_cache: MeasurementCache,

    /// The `compose` method must be called on this widget
    pub(crate) request_compose: bool,
    /// The `compose` method must be called on this widget or a descendant
    pub(crate) needs_compose: bool,

    /// The `pre_paint` method must be called on this widget
    pub(crate) request_pre_paint: bool,
    /// The `paint` method must be called on this widget
    pub(crate) request_paint: bool,
    /// The `post_paint` method must be called on this widget
    pub(crate) request_post_paint: bool,
    /// A painting method must be called on this widget or a descendant
    pub(crate) needs_paint: bool,
    /// How this widget subtree should be represented in the current paint pass.
    pub(crate) paint_layer_mode: PaintLayerMode,

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

    /// This widget or a descendant has `accepts_focus == true`
    pub(crate) descendant_is_focusable: bool,
    /// A focusable widget was added, removed, stashed, disabled, etc.
    pub(crate) needs_update_focusable: bool,

    /// This widget has pending property changes.
    pub(crate) request_update_props: bool,
    /// This widget or a descendant has pending property changes.
    pub(crate) needs_update_props: bool,

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

    // --- PROPERTIES ---
    /// The `PropertyStack` assigned to this widget, if any.
    pub(crate) property_stack_id: Option<PropertyStackId>,
    /// Pending class changes to apply during `run_update_props_pass`.
    pub(crate) class_diff: ClassSetDiff,
    /// Cached property stack resolutions for this widget.
    pub(crate) property_cache: PropertyCache,

    // --- DEBUG INFO ---
    /// The typename of the associated widget.
    ///
    /// Used in some guard rails to provide richer error messages when a parent forgets
    /// to iterate over some children.
    pub(crate) trace_span: Span,
    // TODO - Encapsulate this in WidgetStateDebugInfo struct.
    #[cfg(debug_assertions)]
    pub(crate) widget_name: &'static str,
    #[cfg(debug_assertions)]
    pub(crate) action_type_name: &'static str,
}

impl WidgetState {
    pub(crate) fn new(
        id: WidgetId,
        widget_name: &'static str,
        options: WidgetOptions,
        action_type: TypeId,
        property_stack_id: Option<PropertyStackId>,
        #[cfg(debug_assertions)] action_type_name: &'static str,
    ) -> Self {
        Self {
            id,

            layout_origin: Point::ORIGIN,
            layout_border_box_size: Size::ZERO,
            border_box_insets: Insets::ZERO,
            paint_box_insets: Insets::ZERO,
            visual_border_box: Rect::ZERO,
            bounding_box: Rect::ZERO,
            first_baseline: f64::NAN,
            last_baseline: f64::NAN,
            clip_path: Option::default(),
            transform: options.transform,
            window_transform: Affine::IDENTITY,
            scroll_translation: Vec2::ZERO,
            transform_changed: false,

            action_type,
            accepts_pointer_interaction: true,
            propagates_pointer_interaction: true,
            accepts_focus: false,
            accepts_text_input: false,
            ime_area: None,

            is_new: true,
            is_expecting_place_child_call: false,
            request_layout: true,
            needs_layout: true,
            measurement_cache: MeasurementCache::new(),
            request_compose: true,
            needs_compose: true,
            request_pre_paint: true,
            request_paint: true,
            request_post_paint: true,
            needs_paint: true,
            paint_layer_mode: PaintLayerMode::Inline,
            request_accessibility: true,
            needs_accessibility: true,
            request_anim: true,
            needs_anim: true,
            needs_update_disabled: true,
            needs_update_stashed: true,
            descendant_is_focusable: false,
            needs_update_focusable: true,
            request_update_props: false,
            needs_update_props: false,
            children_changed: true,

            is_explicitly_disabled: options.disabled,
            is_disabled: false,
            is_explicitly_stashed: false,
            is_stashed: false,
            has_hovered: false,
            is_hovered: false,
            has_active: false,
            is_active: false,
            has_focus_target: false,

            property_stack_id,
            class_diff: ClassSetDiff::default(),
            property_cache: PropertyCache::default(),

            trace_span: Span::none(),
            #[cfg(debug_assertions)]
            widget_name,
            #[cfg(debug_assertions)]
            action_type_name,
        }
    }

    /// Updates state to incorporate state changes from a child.
    ///
    /// This method is idempotent and can be called multiple times.
    //
    // TODO: though this method takes child state mutably, child state currently isn't actually
    // mutated anymore. This method may start doing so again in the future, so keep taking &mut for
    // now.
    pub(crate) fn merge_up(&mut self, child_state: &mut Self) {
        if child_state.needs_layout {
            self.measurement_cache.clear();
        }
        self.needs_layout |= child_state.needs_layout;
        self.needs_compose |= child_state.needs_compose;
        self.needs_paint |= child_state.needs_paint;
        self.needs_anim |= child_state.needs_anim;
        self.needs_accessibility |= child_state.needs_accessibility;
        self.needs_update_disabled |= child_state.needs_update_disabled;
        self.children_changed |= child_state.children_changed;
        self.needs_update_focusable |= child_state.needs_update_focusable;
        self.needs_update_stashed |= child_state.needs_update_stashed;
        self.needs_update_props |= child_state.needs_update_props;
    }

    // TODO: Add WidgetState::add_diff method that merges a ClassSetDiff into the WidgetState's class_diff.

    /// Returns `true` if this widget or a descendant explicitly requested layout.
    pub(crate) fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Sets the flag for whether this widget or a descendant explicitly requested layout.
    ///
    /// If set to `true` this also clears the measurement cache.
    pub(crate) fn set_needs_layout(&mut self, needs_layout: bool) {
        if needs_layout {
            self.measurement_cache.clear();
        }
        self.needs_layout = needs_layout;
    }

    /// Returns the widget's layout border-box in the widget's layout border-box coordinate space.
    pub(crate) fn layout_border_box(&self) -> Rect {
        self.layout_border_box_size.to_rect()
    }

    /// Returns the widget's visual content-box in the widget's layout border-box coordinate space.
    pub(crate) fn visual_content_box(&self) -> Rect {
        let x0 = self.visual_border_box.x0 + self.border_box_insets.x0;
        let y0 = self.visual_border_box.y0 + self.border_box_insets.y0;
        let x1 = (self.visual_border_box.x1 - self.border_box_insets.x1).max(x0);
        let y1 = (self.visual_border_box.y1 - self.border_box_insets.y1).max(y0);
        Rect::new(x0, y0, x1, y1)
    }

    /// Returns the widget's layout paint-box in the widget's layout border-box coordinate space.
    pub(crate) fn layout_paint_box(&self) -> Rect {
        self.layout_border_box() + self.paint_box_insets
    }

    /// Returns the widget's visual paint-box in the widget's layout border-box coordinate space.
    pub(crate) fn visual_paint_box(&self) -> Rect {
        self.visual_border_box + self.paint_box_insets
    }

    /// Returns the [`Vec2`] for translating between this widget's
    /// layout and visual coordinate spaces.
    ///
    /// Add this [`Vec2`] to translate from visual to layout,
    /// and subtract this [`Vec2`] to translate from layout to visual.
    pub(crate) fn visual_translation(&self) -> Vec2 {
        Vec2::new(self.visual_border_box.x0, self.visual_border_box.y0)
    }

    /// Returns the [`Vec2`] for translating between this widget's
    /// content-box and border-box coordinate spaces.
    ///
    /// Add this [`Vec2`] to translate from content-box to border-box,
    /// and subtract this [`Vec2`] to translate from border-box to content-box.
    pub(crate) fn border_box_translation(&self) -> Vec2 {
        Vec2::new(self.border_box_insets.x0, self.border_box_insets.y0)
    }

    /// Returns the first baseline relative to the top of the widget's layout border-box.
    pub(crate) fn layout_first_baseline(&self) -> f64 {
        if self.first_baseline.is_nan() {
            self.layout_border_box_size.height
        } else {
            self.first_baseline
        }
    }

    /// Returns the last baseline relative to the top of the widget's layout border-box.
    pub(crate) fn layout_last_baseline(&self) -> f64 {
        if self.last_baseline.is_nan() {
            self.layout_border_box_size.height
        } else {
            self.last_baseline
        }
    }

    /// Returns the area being edited by an IME, in the window's coordinate space.
    ///
    /// If no explicit `ime_area` has been defined this will return the visual border-box
    /// area in the window's coordinate space.
    pub(crate) fn get_ime_area(&self) -> Rect {
        // Note: this returns sensible values for a widget that is translated and/or rescaled.
        // Other transformations like rotation may produce weird IME areas.
        self.window_transform
            .transform_rect_bbox(self.ime_area.unwrap_or(self.visual_border_box))
    }

    /// Returns the result of intersecting the widget's clip path (if any) with the given rect.
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
            || self.needs_update_focusable
            || self.children_changed
            || self.needs_update_props
    }
}
