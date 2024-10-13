// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![cfg(not(tarpaulin_include))]

use std::sync::atomic::{AtomicBool, Ordering};
use vello::kurbo::{Insets, Point, Rect, Size, Vec2};

use crate::{CursorIcon, WidgetId};

// TODO - Sort out names of widget state flags in two categories:
// - request_xxx: means this widget needs the xxx pass to run on it
// - needs_xxx: means this widget or one of its descendants has requested the xxx pass

// FIXME https://github.com/linebender/xilem/issues/376 - Make a note documenting this: the only way to get a &mut WidgetState should be in a pass.
// A pass should reborrow the parent widget state (to avoid crossing wires) and call merge_up at
// the end so that invalidations are always bubbled up.
// Widgets with methods that require invalidation (eg Label::set_text) should take a
// &mut WidgetState as a parameter. Because passes reborrow the parent WidgetState, the only
// way to call such a method is during a pass on the given widget.

// TODO: consider using bitflags for the booleans.

/// Generic state for all widgets in the hierarchy.
///
/// This struct contains the widget's layout rect, flags
/// indicating when the widget is active or focused, and other
/// state necessary for the widget to participate in event
/// flow.
///
/// It is provided to [`paint`] calls as a non-mutable reference,
/// largely so a widget can know its size, also because active
/// and focus state can affect the widget's appearance. Other than
/// that, widgets will generally not interact with it directly,
/// but it is an important part of the [`WidgetPod`] struct.
///
/// [`paint`]: crate::Widget::paint
/// [`WidgetPod`]: crate::WidgetPod
#[derive(Clone, Debug)]
pub(crate) struct WidgetState {
    pub(crate) id: WidgetId,

    // --- LAYOUT ---
    /// The size of the widget; this is the value returned by the widget's layout
    /// method.
    pub(crate) size: Size,
    /// The origin of the widget in the parent's coordinate space; together with
    /// `size` these constitute the widget's layout rect.
    pub(crate) origin: Point,
    /// The origin of the widget in the window coordinate space;
    pub(crate) window_origin: Point,
    /// The insets applied to the layout rect to generate the paint rect.
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    pub(crate) paint_insets: Insets,
    // TODO - Document
    // The computed paint rect, in local coordinates.
    pub(crate) local_paint_rect: Rect,
    /// The offset of the baseline relative to the bottom of the widget.
    ///
    /// In general, this will be zero; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) baseline_offset: f64,

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
    pub(crate) clip: Option<Rect>,

    // TODO - Handle matrix transforms
    pub(crate) translation: Vec2,
    pub(crate) translation_changed: bool,

    // --- PASSES ---
    /// `WidgetAdded` hasn't been sent to this widget yet.
    pub(crate) is_new: bool,

    /// A flag used to track and debug missing calls to `place_child`.
    pub(crate) is_expecting_place_child_call: bool,

    /// This widget explicitly requested layout
    pub(crate) request_layout: bool,
    /// This widget or a descendant explicitly requested layout
    pub(crate) needs_layout: bool,

    /// The compose method must be called on this widget
    pub(crate) request_compose: bool,
    /// The compose method must be called on this widget or a descendant
    pub(crate) needs_compose: bool,

    /// The paint method must be called on this widget
    pub(crate) request_paint: bool,
    /// The paint method must be called on this widget or a descendant
    pub(crate) needs_paint: bool,

    /// The accessibility method must be called on this widget
    pub(crate) request_accessibility: bool,
    /// The accessibility method must be called on this widget or a descendant
    pub(crate) needs_accessibility: bool,

    /// An animation must run on this widget
    pub(crate) request_anim: bool,
    /// An animation must run on this widget or a descendant
    pub(crate) needs_anim: bool,

    /// This widget or a descendant changed its `is_explicitly_disabled` value
    pub(crate) needs_update_disabled: bool,
    /// This widget or a descendant changed its `is_explicitly_stashed` value
    pub(crate) needs_update_stashed: bool,

    pub(crate) update_focus_chain: bool,

    pub(crate) focus_chain: Vec<WidgetId>,

    pub(crate) children_changed: bool,

    // TODO - Remove and handle in WidgetRoot instead
    pub(crate) cursor: Option<CursorIcon>,

    // --- STATUS ---
    /// This widget has been disabled.
    pub(crate) is_explicitly_disabled: bool,
    /// This widget or an ancestor has been disabled.
    pub(crate) is_disabled: bool,

    // TODO - Document concept of "stashing".
    /// This widget has been stashed.
    pub(crate) is_explicitly_stashed: bool,
    /// This widget or an ancestor has been stashed.
    pub(crate) is_stashed: bool,

    pub(crate) is_hovered: bool,

    /// In the focused path, starting from window and ending at the focused widget.
    /// Descendants of the focused widget are not in the focused path.
    pub(crate) has_focus: bool,

    // --- DEBUG INFO ---
    // Used in event/lifecycle/etc methods that are expected to be called recursively
    // on a widget's children, to make sure each child was visited.
    #[cfg(debug_assertions)]
    pub(crate) needs_visit: VisitBool,

    // TODO - document
    #[cfg(debug_assertions)]
    pub(crate) widget_name: &'static str,
}

// This is a hack to have a simple Clone impl for WidgetState
#[derive(Debug)]
pub(crate) struct VisitBool(pub AtomicBool);

impl WidgetState {
    pub(crate) fn new(id: WidgetId, widget_name: &'static str) -> WidgetState {
        WidgetState {
            id,
            origin: Point::ORIGIN,
            window_origin: Point::ORIGIN,
            size: Size::ZERO,
            is_expecting_place_child_call: false,
            paint_insets: Insets::ZERO,
            local_paint_rect: Rect::ZERO,
            accepts_pointer_interaction: true,
            accepts_focus: false,
            accepts_text_input: false,
            ime_area: None,
            clip: Default::default(),
            translation: Vec2::ZERO,
            translation_changed: false,
            is_explicitly_disabled: false,
            is_explicitly_stashed: false,
            is_disabled: false,
            is_stashed: false,
            baseline_offset: 0.0,
            is_new: true,
            is_hovered: false,
            request_layout: true,
            needs_layout: true,
            request_compose: true,
            needs_compose: true,
            request_paint: true,
            needs_paint: true,
            request_accessibility: true,
            needs_accessibility: true,
            has_focus: false,
            request_anim: true,
            needs_anim: true,
            needs_update_disabled: true,
            needs_update_stashed: true,
            focus_chain: Vec::new(),
            children_changed: true,
            cursor: None,
            update_focus_chain: true,
            #[cfg(debug_assertions)]
            needs_visit: VisitBool(false.into()),
            #[cfg(debug_assertions)]
            widget_name,
        }
    }

    /// Create a dummy root state.
    ///
    /// This is useful for passes that need a parent state for the root widget.
    pub(crate) fn synthetic(id: WidgetId, size: Size) -> WidgetState {
        WidgetState {
            size,
            is_new: false,
            needs_layout: false,
            request_compose: false,
            needs_compose: false,
            request_paint: false,
            needs_paint: false,
            request_accessibility: false,
            needs_accessibility: false,
            request_anim: false,
            needs_anim: false,
            needs_update_disabled: false,
            needs_update_stashed: false,
            children_changed: false,
            update_focus_chain: false,
            ..WidgetState::new(id, "<root>")
        }
    }

    pub(crate) fn mark_as_visited(&self, visited: bool) {
        #[cfg(debug_assertions)]
        {
            // TODO - the "!visited" is annoying
            self.needs_visit.0.store(!visited, Ordering::SeqCst);
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn needs_visit(&self) -> bool {
        self.needs_visit.0.load(Ordering::SeqCst)
    }

    /// Update to incorporate state changes from a child.
    ///
    /// This method is idempotent and can be called multiple times.
    //
    // TODO: though this method takes child state mutably, child state currently isn't actually
    // mutated anymore. This method may start doing so again in the future, so keep taking &mut for
    // now.
    pub(crate) fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.needs_layout |= child_state.needs_layout;
        self.needs_compose |= child_state.needs_compose;
        self.needs_paint |= child_state.needs_paint;
        self.needs_anim |= child_state.needs_anim;
        self.needs_accessibility |= child_state.needs_accessibility;
        self.needs_update_disabled |= child_state.needs_update_disabled;
        self.has_focus |= child_state.has_focus;
        self.children_changed |= child_state.children_changed;
        self.update_focus_chain |= child_state.update_focus_chain;
        self.needs_update_stashed |= child_state.needs_update_stashed;
    }

    #[inline]
    pub(crate) fn size(&self) -> Size {
        self.size
    }

    /// The paint region for this widget.
    ///
    /// For more information, see [`WidgetPod::paint_rect`](crate::WidgetPod::paint_rect).
    pub fn paint_rect(&self) -> Rect {
        self.local_paint_rect + self.origin.to_vec2()
    }

    /// The rectangle used when calculating layout with other widgets
    ///
    /// For more information, see [`WidgetPod::layout_rect`](crate::WidgetPod::layout_rect).
    pub fn layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.origin, self.size)
    }

    /// The [`layout_rect`](crate::WidgetPod::layout_rect) in window coordinates.
    ///
    /// This might not map to a visible area of the screen, eg if the widget is scrolled
    /// away.
    pub fn window_layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.window_origin(), self.size)
    }

    /// The clip path of the widget, if any was set.
    ///
    /// For more information, see [`LayoutCtx::set_clip_path`](crate::LayoutCtx::set_clip_path).
    pub fn clip_path(&self) -> Option<Rect> {
        self.clip
    }

    /// Returns the area being edited by an IME, in global coordinates.
    ///
    /// By default, returns the same as [`Self::window_layout_rect`].
    pub(crate) fn get_ime_area(&self) -> Rect {
        self.ime_area.unwrap_or_else(|| self.size.to_rect()) + self.window_origin.to_vec2()
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.window_origin
    }

    pub(crate) fn needs_rewrite_passes(&self) -> bool {
        self.needs_layout
            || self.needs_compose
            || self.needs_paint
            || self.needs_accessibility
            || self.needs_anim
            || self.needs_update_disabled
            || self.needs_update_stashed
    }
}

impl Clone for VisitBool {
    fn clone(&self) -> Self {
        VisitBool(self.0.load(Ordering::SeqCst).into())
    }
}
