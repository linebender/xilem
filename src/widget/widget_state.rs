// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![cfg(not(tarpaulin_include))]

use std::sync::atomic::{AtomicBool, Ordering};

use winit::window::CursorIcon;

use crate::bloom::Bloom;
use crate::kurbo::{Insets, Point, Rect, Size};
use crate::text_helpers::TextFieldRegistration;
use crate::widget::{CursorChange, FocusChange};
use crate::WidgetId;

// FIXME #5 - Make a note documenting this: the only way to get a &mut WidgetState should be in a pass.
// A pass should reborrow the parent widget state (to avoid crossing wires) and call merge_up at
// the end so that invalidations are always bubbled up.
// Widgets with methods that require invalidation (eg Label::set_text) should take a
// &mut WidgetState as a parameter. Because passes reborrow the parent WidgetState, the only
// way to call such a method is during a pass on the given widget.

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
/// [`paint`]: trait.Widget.html#tymethod.paint
/// [`WidgetPod`]: struct.WidgetPod.html
#[derive(Clone, Debug)]
pub struct WidgetState {
    pub(crate) id: WidgetId,

    // --- LAYOUT ---
    /// The size of the child; this is the value returned by the child's layout
    /// method.
    pub(crate) size: Size,
    /// The origin of the child in the parent's coordinate space; together with
    /// `size` these constitute the child's layout rect.
    pub(crate) origin: Point,
    /// The origin of the parent in the window coordinate space;
    pub(crate) parent_window_origin: Point,
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
    // TODO - Document
    pub(crate) is_portal: bool,

    // --- PASSES ---

    // TODO: consider using bitflags for the booleans.
    /// A flag used to track and debug missing calls to place_child.
    pub(crate) is_expecting_place_child_call: bool,

    // True until a WidgetAdded event is received.
    pub(crate) is_new: bool,

    // `true` if a descendent of this widget changed its disabled state and should receive
    // LifeCycle::DisabledChanged or InternalLifeCycle::RouteDisabledChanged
    pub(crate) children_disabled_changed: bool,

    // `true` if this widget has been explicitly disabled, but has not yet seen one of
    // LifeCycle::DisabledChanged or InternalLifeCycle::RouteDisabledChanged
    pub(crate) is_explicitly_disabled_new: bool,

    pub(crate) needs_layout: bool,
    pub(crate) needs_paint: bool,

    /// Because of some scrolling or something, `parent_window_origin` needs to be updated.
    pub(crate) needs_window_origin: bool,

    /// Any descendant has requested an animation frame.
    pub(crate) request_anim: bool,

    pub(crate) update_focus_chain: bool,

    pub(crate) focus_chain: Vec<WidgetId>,
    pub(crate) request_focus: Option<FocusChange>,

    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
    /// The cursor that was set using one of the context methods.
    pub(crate) cursor_change: CursorChange,
    /// The result of merging up children cursors. This gets cleared when merging state up (unlike
    /// cursor_change, which is persistent).
    // TODO - Remove and handle in WidgetRoot instead
    pub(crate) cursor: Option<CursorIcon>,

    pub(crate) text_registrations: Vec<TextFieldRegistration>,

    // --- STATUS ---
    // `true` if one of our ancestors is disabled (meaning we are also disabled).
    pub(crate) ancestor_disabled: bool,

    // `true` if this widget has been explicitly disabled.
    // A widget can be disabled without being *explicitly* disabled if an ancestor is disabled.
    pub(crate) is_explicitly_disabled: bool,

    pub(crate) is_hot: bool,

    pub(crate) is_active: bool,

    /// Any descendant is active.
    pub(crate) has_active: bool,

    /// In the focused path, starting from window and ending at the focused widget.
    /// Descendants of the focused widget are not in the focused path.
    pub(crate) has_focus: bool,

    // TODO - document
    pub(crate) is_stashed: bool,

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
    pub(crate) fn new(id: WidgetId, size: Option<Size>, widget_name: &'static str) -> WidgetState {
        WidgetState {
            id,
            origin: Point::ORIGIN,
            parent_window_origin: Point::ORIGIN,
            size: size.unwrap_or_default(),
            is_expecting_place_child_call: false,
            paint_insets: Insets::ZERO,
            local_paint_rect: Rect::ZERO,
            is_portal: false,
            is_new: true,
            children_disabled_changed: false,
            ancestor_disabled: false,
            is_explicitly_disabled: false,
            baseline_offset: 0.0,
            is_hot: false,
            needs_layout: false,
            needs_paint: false,
            needs_window_origin: false,
            is_active: false,
            has_active: false,
            has_focus: false,
            request_anim: false,
            request_focus: None,
            focus_chain: Vec::new(),
            children: Bloom::new(),
            children_changed: false,
            cursor_change: CursorChange::Default,
            cursor: None,
            is_explicitly_disabled_new: false,
            text_registrations: Vec::new(),
            update_focus_chain: false,
            is_stashed: false,
            #[cfg(debug_assertions)]
            needs_visit: VisitBool(false.into()),
            #[cfg(debug_assertions)]
            widget_name,
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

    pub(crate) fn is_disabled(&self) -> bool {
        self.is_explicitly_disabled || self.ancestor_disabled
    }

    pub(crate) fn tree_disabled_changed(&self) -> bool {
        self.children_disabled_changed
            || self.is_explicitly_disabled != self.is_explicitly_disabled_new
    }

    /// Update to incorporate state changes from a child.
    ///
    /// This will also clear some requests in the child state.
    ///
    /// This method is idempotent and can be called multiple times.
    pub(crate) fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.needs_layout |= child_state.needs_layout;
        self.needs_paint |= child_state.needs_paint;
        self.needs_window_origin |= child_state.needs_window_origin;
        self.request_anim |= child_state.request_anim;
        self.children_disabled_changed |= child_state.children_disabled_changed;
        self.children_disabled_changed |=
            child_state.is_explicitly_disabled_new != child_state.is_explicitly_disabled;
        self.has_active |= child_state.has_active;
        self.has_focus |= child_state.has_focus;
        self.children_changed |= child_state.children_changed;
        self.request_focus = child_state.request_focus.take().or(self.request_focus);
        self.text_registrations
            .append(&mut child_state.text_registrations);
        self.update_focus_chain |= child_state.update_focus_chain;

        // We reset `child_state.cursor` no matter what, so that on the every pass through the tree,
        // things will be recalculated just from `cursor_change`.
        let child_cursor = child_state.take_cursor();
        if let CursorChange::Override(cursor) = &self.cursor_change {
            self.cursor = Some(*cursor);
        } else if child_state.has_active || child_state.is_hot {
            self.cursor = child_cursor;
        }

        if self.cursor.is_none() {
            if let CursorChange::Set(cursor) = &self.cursor_change {
                self.cursor = Some(*cursor);
            }
        }
    }

    /// Because of how cursor merge logic works, we need to handle the leaf case;
    /// in that case there will be nothing in the `cursor` field (as merge_up
    /// is never called) and so we need to also check the `cursor_change` field.
    fn take_cursor(&mut self) -> Option<CursorIcon> {
        self.cursor.take().or_else(|| self.cursor_change.cursor())
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

    /// The [layout_rect](crate::WidgetPod::layout_rect) in window coordinates.
    ///
    /// This might not map to a visible area of the screen, eg if the widget is scrolled
    /// away.
    pub fn window_layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.window_origin(), self.size)
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.parent_window_origin + self.origin.to_vec2()
    }
}

impl Clone for VisitBool {
    fn clone(&self) -> Self {
        VisitBool(self.0.load(Ordering::SeqCst).into())
    }
}
