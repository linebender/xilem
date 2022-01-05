use std::any::Any;
use std::time::Duration;
use tracing::{trace, warn};

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

pub struct WidgetView<'a, 'b, 'w, W: Widget> {
    pub(crate) global_state: &'a mut ContextState<'b>,
    // FIXME - pub
    pub parent_widget_state: &'a mut WidgetState,
    pub widget_state: &'w mut WidgetState,
    pub widget: &'w mut W,
}

impl<W: Widget> Drop for WidgetView<'_, '_, '_, W> {
    fn drop(&mut self) {
        self.parent_widget_state.merge_up(&mut self.widget_state);
    }
}

// ---

// FIXME - This is a big ugly copy-paste.
// Find way to factorize code with impl_context_method

// methods on everyone
impl<W: Widget> WidgetView<'_, '_, '_, W> {
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
        use std::{thread, time};

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

        use std::{thread, time};

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
impl<W: Widget> WidgetView<'_, '_, '_, W> {
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

impl<W: Widget> WidgetView<'_, '_, '_, W> {
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
