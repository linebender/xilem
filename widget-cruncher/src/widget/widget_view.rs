use smallvec::SmallVec;
use std::any::Any;
use std::ops::Deref;
use std::time::Duration;
use tracing::trace;

use crate::command::Command;
use crate::contexts::ContextState;
use crate::ext_event::ExtEventSink;
use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::PietText;
use crate::promise::PromiseToken;
use crate::widget::CursorChange;
use crate::{Target, WidgetPod, WindowId};
use crate::{Widget, WidgetId, WidgetState};
use druid_shell::text::Event as ImeInvalidation;
use druid_shell::{Cursor, TimerToken, WindowHandle};

// TODO - rename lifetimes
pub struct WidgetView<'a, 'b, W: Widget + ?Sized> {
    pub(crate) global_state: &'a mut ContextState<'b>,
    // FIXME - pub
    pub parent_widget_state: &'a mut WidgetState,
    pub widget_state: &'a mut WidgetState,
    pub widget: &'a mut W,
}

impl<W: Widget + ?Sized> Drop for WidgetView<'_, '_, W> {
    fn drop(&mut self) {
        self.parent_widget_state.merge_up(&mut self.widget_state);
    }
}

// ---
pub struct WidgetRef<'w, W: Widget + ?Sized> {
    pub widget_state: &'w WidgetState,
    pub widget: &'w W,
}

impl<'w, W: Widget + ?Sized> Clone for WidgetRef<'w, W> {
    fn clone(&self) -> Self {
        Self {
            widget_state: self.widget_state,
            widget: self.widget,
        }
    }
}

impl<'w, W: Widget + ?Sized> Copy for WidgetRef<'w, W> {}

impl<'w, W: Widget + ?Sized> std::fmt::Debug for WidgetRef<'w, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let widget_name = self.widget.short_type_name();
        let display_name = if let Some(debug_text) = self.widget.get_debug_text() {
            format!("{widget_name}<{debug_text}>").into()
        } else {
            std::borrow::Cow::Borrowed(widget_name)
        };

        let children = self.widget.children();

        if children.is_empty() {
            f.write_str(&display_name)
        } else {
            let mut f_tuple = f.debug_tuple(&display_name);
            for child in children {
                f_tuple.field(&child);
            }
            f_tuple.finish()
        }
    }
}

impl<'w, W: Widget + ?Sized> Deref for WidgetRef<'w, W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

// ---

// TODO - Document
impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    pub fn new(widget_state: &'w WidgetState, widget: &'w W) -> Self {
        WidgetRef {
            widget_state,
            widget,
        }
    }

    pub fn state(self) -> &'w WidgetState {
        self.widget_state
    }

    pub fn widget(self) -> &'w W {
        self.widget
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    // TODO - document
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget,
        }
    }
}

impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    // TODO - document
    pub fn downcast<W2: Widget>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget.as_any().downcast_ref()?,
        })
    }
}

impl<'w> WidgetRef<'w, dyn Widget> {
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        self.widget.children()
    }

    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<WidgetRef<'w, dyn Widget>> {
        if self.state().id == id {
            Some(*self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    pub fn find_widget_at_pos(&self, pos: Point) -> Option<WidgetRef<'w, dyn Widget>> {
        let mut pos = pos;
        let mut innermost_widget: WidgetRef<'w, dyn Widget> = *self;

        if !self.state().layout_rect().contains(pos) {
            return None;
        }

        // FIXME - Handle hidden widgets (eg in scroll areas).
        loop {
            if let Some(child) = innermost_widget.widget().get_child_at_pos(pos) {
                pos -= innermost_widget.state().layout_rect().origin().to_vec2();
                innermost_widget = child;
            } else {
                return Some(innermost_widget);
            }
        }
    }

    // TODO - reorganize this part of the code
    pub(crate) fn prepare_pass(&self) {
        self.state().mark_as_visited(false);
        //self.state.is_expecting_set_origin_call = false;
    }

    // can only be called after on_event and lifecycle
    // checks that basic invariants are held
    pub fn debug_validate(&self, after_layout: bool) {
        if cfg!(not(debug_assertions)) {
            return;
        }

        if self.state().is_new {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget did not receive WidgetAdded",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if self.state().request_focus.is_some()
            || self.state().children_changed
            || !self.state().timers.is_empty()
            || self.state().cursor.is_some()
        {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget state not cleared",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if after_layout && (self.state().needs_layout || self.state().needs_window_origin) {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget layout state not cleared",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        for child in self.widget.children() {
            child.debug_validate(after_layout);

            if !self.state().children.may_contain(&child.state().id) {
                debug_panic!(
                    "Widget '{}' #{} is invalid: child widget '{}' #{} not registered in children filter",
                    self.widget().short_type_name(),
                    self.state().id.to_raw(),
                    child.widget().short_type_name(),
                    child.state().id.to_raw(),
                );
            }
        }
    }
}

// --- Ref logic ---

impl<'a, 'b, W: Widget> WidgetView<'a, 'b, W> {
    pub fn as_dyn(&mut self) -> WidgetView<'_, 'b, dyn Widget> {
        WidgetView {
            global_state: self.global_state,
            parent_widget_state: self.parent_widget_state,
            widget_state: self.widget_state,
            widget: self.widget,
        }
    }
}

impl<'a, 'b, W: Widget + ?Sized> WidgetView<'a, 'b, W> {
    pub fn downcast<W2: Widget>(&mut self) -> Option<WidgetView<'_, 'b, W2>> {
        Some(WidgetView {
            global_state: self.global_state,
            parent_widget_state: self.parent_widget_state,
            widget_state: self.widget_state,
            widget: self.widget.as_mut_any().downcast_mut()?,
        })
    }
}

// TODO - remove
impl<'a, 'b> WidgetView<'a, 'b, Box<dyn Widget>> {
    pub fn downcast_box<W2: Widget>(&mut self) -> Option<WidgetView<'_, 'b, W2>> {
        Some(WidgetView {
            global_state: self.global_state,
            parent_widget_state: self.parent_widget_state,
            widget_state: self.widget_state,
            widget: (&mut **self.widget).as_mut_any().downcast_mut()?,
        })
    }
}

// -
// -
// -
// FIXME - This is a big ugly copy-paste.
// Find way to factorize code with impl_context_method
// -
// -
// -
// methods on everyone
impl<W: Widget + ?Sized> WidgetView<'_, '_, W> {
    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_state.id
    }

    /// Returns a reference to the current `WindowHandle`.
    pub fn window(&self) -> &WindowHandle {
        self.global_state.window
    }

    /// Get the `WindowId` of the current window.
    pub fn window_id(&self) -> WindowId {
        self.global_state.window_id
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut PietText {
        &mut self.global_state.text
    }

    pub fn run_in_background(
        &mut self,
        background_task: impl FnOnce(ExtEventSink) + Send + 'static,
    ) {
        use std::thread;

        let ext_event_sink = self.global_state.ext_event_sink.clone();
        thread::spawn(move || {
            background_task(ext_event_sink);
        });
    }

    // TODO - should take FnOnce.
    pub fn compute_in_background<T: Any + Send>(
        &mut self,
        background_task: impl Fn(ExtEventSink) -> T + Send + 'static,
    ) -> PromiseToken<T> {
        let token = PromiseToken::<T>::new();

        use std::thread;

        let ext_event_sink = self.global_state.ext_event_sink.clone();
        let widget_id = self.widget_state.id;
        let window_id = self.global_state.window_id;
        thread::spawn(move || {
            let result = background_task(ext_event_sink.clone());
            // TODO unwrap_or
            let _ = ext_event_sink.resolve_promise(token.make_result(result), widget_id, window_id);
        });

        token
    }

    // TODO - document
    pub fn skip_child(&self, child: &mut WidgetPod<impl Widget>) {
        child.mark_as_visited();
    }
}

// methods on everyone but layoutctx
impl<W: Widget + ?Sized> WidgetView<'_, '_, W> {
    /// The layout size.
    ///
    /// This is the layout size as ultimately determined by the parent
    /// container, on the previous layout pass.
    ///
    /// Generally it will be the same as the size returned by the child widget's
    /// [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn size(&self) -> Size {
        self.widget_state.size()
    }

    /// The origin of the widget in window coordinates, relative to the top left corner of the
    /// content area.
    pub fn window_origin(&self) -> Point {
        self.widget_state.window_origin()
    }

    /// Convert a point from the widget's coordinate space to the window's.
    ///
    /// The returned point is relative to the content area; it excludes window chrome.
    pub fn to_window(&self, widget_point: Point) -> Point {
        self.window_origin() + widget_point.to_vec2()
    }

    /// Convert a point from the widget's coordinate space to the screen's.
    /// See the [`Screen`] module
    ///
    /// [`Screen`]: druid_shell::Screen
    pub fn to_screen(&self, widget_point: Point) -> Point {
        let insets = self.window().content_insets();
        let content_origin = self.window().get_position() + Vec2::new(insets.x0, insets.y0);
        content_origin + self.to_window(widget_point).to_vec2()
    }

    /// The "hot" (aka hover) status of a widget.
    ///
    /// A widget is "hot" when the mouse is hovered over it. Widgets will
    /// often change their appearance as a visual indication that they
    /// will respond to mouse interaction.
    ///
    /// The hot status is computed from the widget's layout rect. In a
    /// container hierarchy, all widgets with layout rects containing the
    /// mouse position have hot status.
    ///
    /// Discussion: there is currently some confusion about whether a
    /// widget can be considered hot when some other widget is active (for
    /// example, when clicking to one widget and dragging to the next).
    /// The documentation should clearly state the resolution.
    pub fn is_hot(&self) -> bool {
        self.widget_state.is_hot
    }

    /// The active status of a widget.
    ///
    /// Active status generally corresponds to a mouse button down. Widgets
    /// with behavior similar to a button will call [`set_active`] on mouse
    /// down and then up.
    ///
    /// When a widget is active, it gets mouse events even when the mouse
    /// is dragged away.
    ///
    /// [`set_active`]: struct.EventCtx.html#method.set_active
    pub fn is_active(&self) -> bool {
        self.widget_state.is_active
    }

    /// The focus status of a widget.
    ///
    /// Returns `true` if this specific widget is focused.
    /// To check if any descendants are focused use [`has_focus`].
    ///
    /// Focus means that the widget receives keyboard events.
    ///
    /// A widget can request focus using the [`request_focus`] method.
    /// It's also possible to register for automatic focus via [`register_for_focus`].
    ///
    /// If a widget gains or loses focus it will get a [`LifeCycle::FocusChanged`] event.
    ///
    /// Only one widget at a time is focused. However due to the way events are routed,
    /// all ancestors of that widget will also receive keyboard events.
    ///
    /// [`request_focus`]: struct.EventCtx.html#method.request_focus
    /// [`register_for_focus`]: struct.LifeCycleCtx.html#method.register_for_focus
    /// [`LifeCycle::FocusChanged`]: enum.LifeCycle.html#variant.FocusChanged
    /// [`has_focus`]: #method.has_focus
    pub fn is_focused(&self) -> bool {
        self.global_state.focus_widget == Some(self.widget_id())
    }

    /// The (tree) focus status of a widget.
    ///
    /// Returns `true` if either this specific widget or any one of its descendants is focused.
    /// To check if only this specific widget is focused use [`is_focused`],
    ///
    /// [`is_focused`]: #method.is_focused
    pub fn has_focus(&self) -> bool {
        self.widget_state.has_focus
    }

    /// The disabled state of a widget.
    ///
    /// Returns `true` if this widget or any of its ancestors is explicitly disabled.
    /// To make this widget explicitly disabled use [`set_disabled`].
    ///
    /// Disabled means that this widget should not change the state of the application. What
    /// that means is not entirely clear but in any it should not change its data. Therefore
    /// others can use this as a safety mechanism to prevent the application from entering an
    /// illegal state.
    /// For an example the decrease button of a counter of type `usize` should be disabled if the
    /// value is `0`.
    ///
    /// [`set_disabled`]: EventCtx::set_disabled
    pub fn is_disabled(&self) -> bool {
        self.widget_state.is_disabled()
    }
}

impl<W: Widget + ?Sized> WidgetView<'_, '_, W> {
    /// Set the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. If a child widget also sets a
    /// cursor, the child widget's cursor will take precedence. (If that isn't what you want, use
    /// [`override_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        trace!("set_cursor {:?}", cursor);
        self.widget_state.cursor_change = CursorChange::Set(cursor.clone());
    }

    /// Override the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. This will override the cursor
    /// preferences of a child widget. (If that isn't what you want, use [`set_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`set_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn override_cursor(&mut self, cursor: &Cursor) {
        trace!("override_cursor {:?}", cursor);
        self.widget_state.cursor_change = CursorChange::Override(cursor.clone());
    }

    /// Clear the cursor icon.
    ///
    /// This undoes the effect of [`set_cursor`] and [`override_cursor`].
    ///
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`set_cursor`]: EventCtx::set_cursor
    pub fn clear_cursor(&mut self) {
        trace!("clear_cursor");
        self.widget_state.cursor_change = CursorChange::Default;
    }

    // methods on event, update, and lifecycle
    /// Request a [`paint`] pass. This is equivalent to calling
    /// [`request_paint_rect`] for the widget's [`paint_rect`].
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    /// [`request_paint_rect`]: #method.request_paint_rect
    /// [`paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn request_paint(&mut self) {
        trace!("request_paint");
        self.widget_state.invalid.set_rect(
            self.widget_state.paint_rect() - self.widget_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given
    /// relative to our layout rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        trace!("request_paint_rect {}", rect);
        self.widget_state.invalid.add_rect(rect);
    }

    /// Request a layout pass.
    ///
    /// A Widget's [`layout`] method is always called when the widget tree
    /// changes, or the window is resized.
    ///
    /// If your widget would like to have layout called at any other time,
    /// (such as if it would like to change the layout of children in
    /// response to some event) it must call this method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn request_layout(&mut self) {
        trace!("request_layout");
        self.widget_state.needs_layout = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        trace!("request_anim_frame");
        self.widget_state.request_anim = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child, removing a child or changing which
    /// children are hidden (see [`should_propagate_to_hidden`]).
    ///
    /// [`should_propagate_to_hidden`]: crate::Event::should_propagate_to_hidden
    pub fn children_changed(&mut self) {
        trace!("children_changed");
        self.widget_state.children_changed = true;
        self.widget_state.update_focus_chain = true;
        self.request_layout();
    }

    /// Set the disabled state for this widget.
    ///
    /// Setting this to `false` does not mean a widget is not still disabled; for instance it may
    /// still be disabled by an ancestor. See [`is_disabled`] for more information.
    ///
    /// Calling this method during [`LifeCycle::DisabledChanged`] has no effect.
    ///
    /// [`LifeCycle::DisabledChanged`]: struct.LifeCycle.html#variant.DisabledChanged
    /// [`is_disabled`]: EventCtx::is_disabled
    pub fn set_disabled(&mut self, disabled: bool) {
        // widget_state.children_disabled_changed is not set because we want to be able to delete
        // changes that happened during DisabledChanged.
        self.widget_state.is_explicitly_disabled_new = disabled;
    }

    /// Indicate that text input state has changed.
    ///
    /// A widget that accepts text input should call this anytime input state
    /// (such as the text or the selection) changes as a result of a non text-input
    /// event.
    pub fn invalidate_text_input(&mut self, event: ImeInvalidation) {
        let payload = crate::command::ImeInvalidation {
            widget: self.widget_id(),
            event,
        };
        let cmd = crate::command::INVALIDATE_IME
            .with(payload)
            .to(Target::Window(self.window_id()));
        self.submit_command(cmd);
    }

    // methods on everyone but paintctx
    /// Submit a [`Command`] to be run after this event is handled.
    ///
    /// Commands are run in the order they are submitted; all commands
    /// submitted during the handling of an event are executed before
    /// the [`update`] method is called; events submitted during [`update`]
    /// are handled after painting.
    ///
    /// [`Target::Auto`] commands will be sent to the window containing the widget.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update`]: trait.Widget.html#tymethod.update
    pub fn submit_command(&mut self, cmd: impl Into<Command>) {
        trace!("submit_command");
        self.global_state.submit_command(cmd.into())
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
        trace!("request_timer deadline={:?}", deadline);
        self.global_state
            .request_timer(&mut self.widget_state, deadline)
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::widget::{Button, Label};
    use crate::{Widget, WidgetPod};

    #[test]
    fn downcast_ref() {
        let label = WidgetPod::new(Label::new("Hello"));
        let dyn_widget: WidgetRef<dyn Widget> = label.as_dyn();

        let label = dyn_widget.downcast::<Label>();
        assert_matches!(label, Some(_));
        let label = dyn_widget.downcast::<Button>();
        assert_matches!(label, None);
    }

    // TODO - downcast_view
}
