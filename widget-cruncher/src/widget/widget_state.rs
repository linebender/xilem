use std::collections::HashMap;
use tracing::{info_span, trace, warn};

use crate::bloom::Bloom;
use crate::contexts::ContextState;
use crate::kurbo::{Affine, Insets, Point, Rect, Shape, Size, Vec2};
use crate::text::{TextFieldRegistration, TextLayout};
use crate::util::ExtendDrain;
use crate::widget::{CursorChange, FocusChange};
use crate::{
    ArcStr, BoxConstraints, Color, Cursor, Env, Event, EventCtx, InternalEvent, InternalLifeCycle,
    LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Region, RenderContext, TimerToken, Widget,
    WidgetId,
};

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
#[derive(Clone)]
pub struct WidgetState {
    pub(crate) id: WidgetId,
    /// The size of the child; this is the value returned by the child's layout
    /// method.
    pub(crate) size: Size,
    /// The origin of the child in the parent's coordinate space; together with
    /// `size` these constitute the child's layout rect.
    pub(crate) origin: Point,
    /// The origin of the parent in the window coordinate space;
    pub(crate) parent_window_origin: Point,
    /// A flag used to track and debug missing calls to set_origin.
    pub(crate) is_expecting_set_origin_call: bool,
    /// The insets applied to the layout rect to generate the paint rect.
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    pub(crate) paint_insets: Insets,

    /// The offset of the baseline relative to the bottom of the widget.
    ///
    /// In general, this will be zero; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) baseline_offset: f64,

    // The region that needs to be repainted, relative to the widget's bounds.
    pub(crate) invalid: Region,

    // The part of this widget that is visible on the screen is offset by this
    // much. This will be non-zero for widgets that are children of `Scroll`, or
    // similar, and it is used for propagating invalid regions.
    pub(crate) viewport_offset: Vec2,

    // TODO: consider using bitflags for the booleans.

    // True until a WidgetAdded event is received.
    pub(crate) is_new: bool,

    // `true` if a descendent of this widget changed its disabled state and should receive
    // LifeCycle::DisabledChanged or InternalLifeCycle::RouteDisabledChanged
    pub(crate) children_disabled_changed: bool,

    // `true` if one of our ancestors is disabled (meaning we are also disabled).
    pub(crate) ancestor_disabled: bool,

    // `true` if this widget has been explicitly disabled.
    // A widget can be disabled without being *explicitly* disabled if an ancestor is disabled.
    pub(crate) is_explicitly_disabled: bool,

    // `true` if this widget has been explicitly disabled, but has not yet seen one of
    // LifeCycle::DisabledChanged or InternalLifeCycle::RouteDisabledChanged
    pub(crate) is_explicitly_disabled_new: bool,

    pub(crate) is_hot: bool,

    pub(crate) is_active: bool,

    pub(crate) needs_layout: bool,

    /// Because of some scrolling or something, `parent_window_origin` needs to be updated.
    pub(crate) needs_window_origin: bool,

    /// Any descendant is active.
    pub(crate) has_active: bool,

    /// In the focused path, starting from window and ending at the focused widget.
    /// Descendants of the focused widget are not in the focused path.
    pub(crate) has_focus: bool,

    /// Any descendant has requested an animation frame.
    pub(crate) request_anim: bool,

    /// Any descendant has requested update.
    pub(crate) request_update: bool,

    pub(crate) update_focus_chain: bool,

    pub(crate) focus_chain: Vec<WidgetId>,
    pub(crate) request_focus: Option<FocusChange>,
    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
    /// Associate timers with widgets that requested them.
    pub(crate) timers: HashMap<TimerToken, WidgetId>,
    /// The cursor that was set using one of the context methods.
    pub(crate) cursor_change: CursorChange,
    /// The result of merging up children cursors. This gets cleared when merging state up (unlike
    /// cursor_change, which is persistent).
    pub(crate) cursor: Option<Cursor>,

    pub(crate) text_registrations: Vec<TextFieldRegistration>,

    // Used in event/lifecycle/etc methods that are expected to be called recursively
    // on a widget's children, to make sure each child was visited.
    #[cfg(debug_assertions)]
    pub(crate) was_visited: bool,
}

impl WidgetState {
    pub(crate) fn new(id: WidgetId, size: Option<Size>) -> WidgetState {
        WidgetState {
            id,
            origin: Point::ORIGIN,
            parent_window_origin: Point::ORIGIN,
            size: size.unwrap_or_default(),
            is_expecting_set_origin_call: true,
            paint_insets: Insets::ZERO,
            invalid: Region::EMPTY,
            viewport_offset: Vec2::ZERO,
            is_new: true,
            children_disabled_changed: false,
            ancestor_disabled: false,
            is_explicitly_disabled: false,
            baseline_offset: 0.0,
            is_hot: false,
            needs_layout: false,
            needs_window_origin: false,
            is_active: false,
            has_active: false,
            has_focus: false,
            request_anim: false,
            request_update: false,
            request_focus: None,
            focus_chain: Vec::new(),
            children: Bloom::new(),
            children_changed: false,
            timers: HashMap::new(),
            cursor_change: CursorChange::Default,
            cursor: None,
            is_explicitly_disabled_new: false,
            text_registrations: Vec::new(),
            update_focus_chain: false,
            was_visited: false,
        }
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.is_explicitly_disabled || self.ancestor_disabled
    }

    pub(crate) fn tree_disabled_changed(&self) -> bool {
        self.children_disabled_changed
            || self.is_explicitly_disabled != self.is_explicitly_disabled_new
    }

    pub(crate) fn add_timer(&mut self, timer_token: TimerToken) {
        self.timers.insert(timer_token, self.id);
    }

    /// Update to incorporate state changes from a child.
    ///
    /// This will also clear some requests in the child state.
    ///
    /// This method is idempotent and can be called multiple times.
    pub(crate) fn merge_up(&mut self, child_state: &mut WidgetState) {
        let clip = self
            .layout_rect()
            .with_origin(Point::ORIGIN)
            .inset(self.paint_insets);
        let offset = child_state.layout_rect().origin().to_vec2() - child_state.viewport_offset;
        for &r in child_state.invalid.rects() {
            let r = (r + offset).intersect(clip);
            if r.area() != 0.0 {
                self.invalid.add_rect(r);
            }
        }
        // Clearing the invalid rects here is less fragile than doing it while painting. The
        // problem is that widgets (for example, Either) might choose not to paint certain
        // invisible children, and we shouldn't allow these invisible children to accumulate
        // invalid rects.
        child_state.invalid.clear();

        self.needs_layout |= child_state.needs_layout;
        self.needs_window_origin |= child_state.needs_window_origin;
        self.request_anim |= child_state.request_anim;
        self.children_disabled_changed |= child_state.children_disabled_changed;
        self.children_disabled_changed |=
            child_state.is_explicitly_disabled_new != child_state.is_explicitly_disabled;
        self.has_active |= child_state.has_active;
        self.has_focus |= child_state.has_focus;
        self.children_changed |= child_state.children_changed;
        self.request_update |= child_state.request_update;
        self.request_focus = child_state.request_focus.take().or(self.request_focus);
        self.timers.extend_drain(&mut child_state.timers);
        self.text_registrations
            .append(&mut child_state.text_registrations);
        self.update_focus_chain |= child_state.update_focus_chain;

        // We reset `child_state.cursor` no matter what, so that on the every pass through the tree,
        // things will be recalculated just from `cursor_change`.
        let child_cursor = child_state.take_cursor();
        if let CursorChange::Override(cursor) = &self.cursor_change {
            self.cursor = Some(cursor.clone());
        } else if child_state.has_active || child_state.is_hot {
            self.cursor = child_cursor;
        }

        if self.cursor.is_none() {
            if let CursorChange::Set(cursor) = &self.cursor_change {
                self.cursor = Some(cursor.clone());
            }
        }
    }

    /// Because of how cursor merge logic works, we need to handle the leaf case;
    /// in that case there will be nothing in the `cursor` field (as merge_up
    /// is never called) and so we need to also check the `cursor_change` field.
    fn take_cursor(&mut self) -> Option<Cursor> {
        self.cursor.take().or_else(|| self.cursor_change.cursor())
    }

    #[inline]
    pub(crate) fn size(&self) -> Size {
        self.size
    }

    /// The paint region for this widget.
    ///
    /// For more information, see [`WidgetPod::paint_rect`].
    ///
    /// [`WidgetPod::paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn paint_rect(&self) -> Rect {
        self.layout_rect() + self.paint_insets
    }

    /// The rectangle used when calculating layout with other widgets
    pub fn layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.origin, self.size)
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.parent_window_origin + self.origin.to_vec2() - self.viewport_offset
    }
}
