// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Events.

use druid_shell::{Clipboard, KeyEvent, TimerToken};

use crate::kurbo::{Rect, Size};
use crate::mouse::MouseEvent;
// TODO
use crate::promise::PromiseResult;
use crate::{Command, Notification, WidgetId};

// TODO
// An important category is events plumbed from the platform windowing
// system, which includes mouse and keyboard events, but also (in the
// future) status changes such as window focus changes.

/// An event, propagated downwards during event flow.
///
/// Events are things that happen that the UI can be expected to react to:
///
/// - Conventional platform interactions (eg [`MouseEvent`], [`KeyEvent`]).
/// - Messages sent from other widgets or background threads ([`Command`] and
/// [`Notification`]).
/// - Responses to requests send by the widget ([`Event::Timer`] and [`PromiseResult`]).
///
/// Events are propagated through "event flow": they are passed down the
/// widget tree through [`Widget::on_event`](crate::Widget::on_event) methods.
/// Container widgets will generally pass each event to their children through
/// [`WidgetPod::on_event`](crate::WidgetPod::on_event), which internally takes
/// care of most of the event flow logic (in particular whether or not to recurse).
///
/// This enum is expected to grow considerably, as there are many, many
/// different kinds of events that are relevant in a GUI.
// TODO - Add tutorial about event flow
// TODO - Normalize variant decriptions
// TODO - Migrate bulk of descriptions to other types
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Event {
    /// Sent to all widgets in a given window when that window is first instantiated.
    ///
    /// This should always be the first `Event` received, although widgets will
    /// receive [`LifeCycle::WidgetAdded`] first.
    ///
    /// Widgets should handle this event if they need to do some addition setup
    /// when a window is first created.
    WindowConnected,

    /// Sent to all widgets in a given window when the system requests to close the window.
    ///
    /// If the event is handled (with [`EventCtx::set_handled`](crate::EventCtx::set_handled)), the window will not be closed.
    /// All widgets are given an opportunity to handle this event; your widget should not assume
    /// that the window *will* close just because this event is received; for instance, you should
    /// avoid destructive side effects such as cleaning up resources.
    WindowCloseRequested,

    /// Sent to all widgets in a given window when the system is going to close that window.
    ///
    /// This event means the window *will* go away; it is safe to dispose of resources and
    /// do any other cleanup.
    WindowDisconnected,

    /// Called on the root widget when the window size changes.
    ///
    /// **Note:** it's not obvious this should be propagated to user
    /// widgets. It might be better to just handle it in `layout`.
    WindowSize(Size),

    /// Called when a mouse button is pressed.
    MouseDown(MouseEvent),

    /// Called when a mouse button is released.
    MouseUp(MouseEvent),

    /// Called when the mouse is moved.
    ///
    /// The `MouseMove` event is propagated to the active widget, if
    /// there is one, otherwise to hot widgets (see `HotChanged`).
    /// If a widget loses its hot status due to `MouseMove` then that specific
    /// `MouseMove` event is also still sent to that widget.
    ///
    /// The `MouseMove` event is also the primary mechanism for widgets
    /// to set a cursor, for example to an I-bar inside a text widget. A
    /// simple tactic is for the widget to unconditionally call
    /// [`set_cursor`] in the MouseMove handler, as `MouseMove` is only
    /// propagated to active or hot widgets.
    ///
    /// [`set_cursor`]: struct.EventCtx.html#method.set_cursor
    MouseMove(MouseEvent),

    // TODO - What about trackpad scrolling? Touchscreens?
    /// Called when the mouse wheel or trackpad is scrolled.
    Wheel(MouseEvent),

    /// Called when a key is pressed.
    KeyDown(KeyEvent),

    /// Called when a key is released.
    ///
    /// Because of repeat, there may be a number `KeyDown` events before
    /// a corresponding `KeyUp` is sent.
    KeyUp(KeyEvent),

    /// Called when a paste command is received.
    Paste(Clipboard),

    // TODO - Rename to "TextChange" or something similar?
    /// Sent to a widget when the platform may have mutated shared IME state.
    ///
    /// This is sent to a widget that has an attached IME session anytime the
    /// platform has released a mutable lock on shared state.
    ///
    /// This does not *mean* that any state has changed, but the widget
    /// should check the shared state, perform invalidation, and update `Data`
    /// as necessary.
    ImeStateChange,

    /// Called when the trackpad is pinched.
    ///
    /// The value is a delta.
    Zoom(f64),

    /// Called at the beginning of a new animation frame.
    ///
    /// On the first frame when transitioning from idle to animating, `interval`
    /// will be 0. (This logic is presently per-window but might change to
    /// per-widget to make it more consistent). Otherwise it is in nanoseconds.
    ///
    /// The `paint` method will be called shortly after this event is finished.
    /// As a result, you should try to avoid doing anything computationally
    /// intensive in response to an `AnimFrame` event: it might make the app miss
    /// the monitor's refresh, causing lag or jerky animations.
    AnimFrame(u64),

    /// Called on a timer event.
    ///
    /// When the user creates a timer through
    /// [`EventCtx::request_timer`](crate::EventCtx::request_timer),
    /// a `Timer` event is sent when the time is up.
    ///
    /// Note that timer events from other widgets may be delivered as well. Use
    /// the token returned from the `request_timer()` call to filter events more
    /// precisely.
    Timer(TimerToken),

    /// Called when a promise returns.
    ///
    /// When the user creates a promise through
    /// [`EventCtx::compute_in_background`](crate::EventCtx::compute_in_background),
    /// a`PromiseResult` event is sent when the computation completes.
    PromiseResult(PromiseResult),

    /// An event containing a [`Command`] to be handled by the widget.
    ///
    /// Commands are messages, optionally with attached data, from other
    /// widgets or other sources. See [`Command`] for details.
    Command(Command),

    /// A [`Notification`] from one of this widget's descendants.
    ///
    /// Notifications are messages, optionally with attached data, from child
    /// widgets. See [`Notification`] for details.
    ///
    /// If you handle a [`Notification`], you should call
    /// [`EventCtx::set_handled`](crate::EventCtx::set_handled)
    /// to stop the notification from being delivered to further ancestors.
    Notification(Notification),

    /// Internal Masonry event.
    ///
    /// This should always be passed down to descendant [`WidgetPod`]s.
    ///
    /// [`WidgetPod`]: struct.WidgetPod.html
    Internal(InternalEvent),
}

/// Internal events used by Masonry inside [`WidgetPod`].
///
/// These events are translated into regular [`Event`]s
/// and should not be used directly.
///
/// [`WidgetPod`]: struct.WidgetPod.html
/// [`Event`]: enum.Event.html
#[derive(Debug, Clone)]
pub enum InternalEvent {
    /// Sent in some cases when the mouse has left the window.
    ///
    /// This is used in cases when the platform no longer sends mouse events,
    /// but we know that we've stopped receiving the mouse events.
    MouseLeave,

    /// A command still in the process of being dispatched.
    TargetedCommand(Command),

    /// Used for routing timer events.
    RouteTimer(TimerToken, WidgetId),

    /// Used for routing promise results.
    RoutePromiseResult(PromiseResult, WidgetId),

    /// Route an IME change event.
    RouteImeStateChange(WidgetId),
}

/// Application life cycle events.
///
/// Unlike [`Event`]s, [`LifeCycle`] events are generated by Masonry, and
/// may occur at different times during a given pass of the event loop. The
/// [`LifeCycle::WidgetAdded`] event, for instance, may occur when the app
/// first launches (during the handling of [`Event::WindowConnected`]) or it
/// may occur during an [`on_event`](crate::Widget::on_event) pass, if some
/// widget has been added then.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum LifeCycle {
    /// Sent to a `Widget` when it is added to the widget tree. This should be
    /// the first message that each widget receives.
    ///
    /// Widgets should handle this event in order to do any initial setup.
    ///
    /// In addition to setup, this event is also used by the framework to
    /// track certain types of important widget state.
    ///
    /// ## Registering children
    ///
    /// Container widgets (widgets which use [`WidgetPod`](crate::WidgetPod) to
    /// manage children) must ensure that this event is forwarded to those children.
    /// The [`WidgetPod`](crate::WidgetPod) itself will handle registering those
    /// children with the system; this is required for things like correct routing
    /// of events.
    WidgetAdded,

    // TODO - Put in StatusChange
    /// Called when the Disabled state of the widgets is changed.
    ///
    /// To check if a widget is disabled, see [`is_disabled`].
    ///
    /// To change a widget's disabled state, see [`set_disabled`].
    ///
    /// [`is_disabled`]: crate::EventCtx::is_disabled
    /// [`set_disabled`]: crate::EventCtx::set_disabled
    DisabledChanged(bool),

    /// Called when the widget tree changes and Masonry wants to rebuild the
    /// Focus-chain.
    ///
    /// It is the only place from which [`register_for_focus`] should be called.
    /// By doing so the widget can get focused by other widgets using [`focus_next`] or [`focus_prev`].
    ///
    /// [`register_for_focus`]: crate::LifeCycleCtx::register_for_focus
    /// [`focus_next`]: crate::EventCtx::focus_next
    /// [`focus_prev`]: crate::EventCtx::focus_prev
    BuildFocusChain,

    /// Called when a child widgets uses
    /// [`EventCtx::request_pan_to_this`](crate::EventCtx::request_pan_to_this).
    RequestPanToChild(Rect),

    /// Internal Masonry lifecycle event.
    ///
    /// This should always be passed down to descendant [`WidgetPod`]s.
    ///
    /// [`WidgetPod`]: struct.WidgetPod.html
    Internal(InternalLifeCycle),
}

/// Internal lifecycle events used by Masonry inside [`WidgetPod`].
///
/// These events are translated into regular [`LifeCycle`] events
/// and should not be used directly.
///
/// [`WidgetPod`]: struct.WidgetPod.html
/// [`LifeCycle`]: enum.LifeCycle.html
#[derive(Debug, Clone)]
pub enum InternalLifeCycle {
    /// Used to route the `WidgetAdded` event to the required widgets.
    RouteWidgetAdded,

    /// Used to route the `FocusChanged` event.
    RouteFocusChanged {
        /// the widget that is losing focus, if any
        old: Option<WidgetId>,
        /// the widget that is gaining focus, if any
        new: Option<WidgetId>,
    },

    /// Used to route the `DisabledChanged` event to the required widgets.
    RouteDisabledChanged,

    /// The parents widget origin in window coordinate space has changed.
    ParentWindowOrigin,
}

/// Event indicating status changes within the widget hierarchy.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum StatusChange {
    /// Called when the "hot" status changes.
    ///
    /// This will always be called _before_ the event that triggered it; that is,
    /// when the mouse moves over a widget, that widget will receive
    /// `StatusChange::HotChanged` before it receives `Event::MouseMove`.
    ///
    /// See [`is_hot`](struct.EventCtx.html#method.is_hot) for
    /// discussion about the hot status.
    HotChanged(bool),

    /// Called when the focus status changes.
    ///
    /// This will always be called immediately after a new widget gains focus.
    /// The newly focused widget will receive this with `true` and the widget
    /// that lost focus will receive this with `false`.
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    FocusChanged(bool),
}

impl Event {
    /// Whether this event should be sent to widgets which are currently not visible and not
    /// accessible.
    ///
    /// For example: the hidden tabs in a tabs widget are `hidden` whereas the non-visible
    /// widgets in a scroll are not, since you can bring them into view by scrolling.
    ///
    /// This distinction between scroll and tabs is due to one of the main purposes of
    /// this method: determining which widgets are allowed to receive focus. As a rule
    /// of thumb a widget counts as `hidden` if it makes no sense for it to receive focus
    /// when the user presses thee 'tab' key.
    ///
    /// If a widget changes which children are hidden it must call [`children_changed`].
    ///
    /// See also [`LifeCycle::should_propagate_to_hidden`].
    ///
    /// [`children_changed`]: crate::EventCtx::children_changed
    /// [`LifeCycle::should_propagate_to_hidden`]: LifeCycle::should_propagate_to_hidden
    pub fn should_propagate_to_hidden(&self) -> bool {
        match self {
            Event::WindowConnected
            | Event::WindowCloseRequested
            | Event::WindowDisconnected
            | Event::WindowSize(_)
            | Event::Timer(_)
            | Event::AnimFrame(_)
            | Event::Command(_)
            | Event::PromiseResult(_)
            | Event::Notification(_)
            | Event::Internal(_) => true,
            Event::MouseDown(_)
            | Event::MouseUp(_)
            | Event::MouseMove(_)
            | Event::Wheel(_)
            | Event::KeyDown(_)
            | Event::KeyUp(_)
            | Event::Paste(_)
            | Event::ImeStateChange
            | Event::Zoom(_) => false,
        }
    }

    /// Short name, for debug logging.
    ///
    /// Essentially returns the enum variant name.
    pub fn short_name(&self) -> &'static str {
        match self {
            Event::Internal(internal) => match internal {
                InternalEvent::MouseLeave => "MouseLeave",
                InternalEvent::TargetedCommand(_) => "TargetedCommand",
                InternalEvent::RouteTimer(_, _) => "RouteTimer",
                InternalEvent::RoutePromiseResult(_, _) => "RoutePromiseResult",
                InternalEvent::RouteImeStateChange(_) => "RouteImeStateChange",
            },
            Event::WindowConnected => "WindowConnected",
            Event::WindowCloseRequested => "WindowCloseRequested",
            Event::WindowDisconnected => "WindowDisconnected",
            Event::WindowSize(_) => "WindowSize",
            Event::Timer(_) => "Timer",
            Event::AnimFrame(_) => "AnimFrame",
            Event::Command(_) => "Command",
            Event::PromiseResult(_) => "PromiseResult",
            Event::Notification(_) => "Notification",
            Event::MouseDown(_) => "MouseDown",
            Event::MouseUp(_) => "MouseUp",
            Event::MouseMove(_) => "MouseMove",
            Event::Wheel(_) => "Wheel",
            Event::KeyDown(_) => "KeyDown",
            Event::KeyUp(_) => "KeyUp",
            Event::Paste(_) => "Paste",
            Event::ImeStateChange => "ImeStateChange",
            Event::Zoom(_) => "Zoom",
        }
    }
}

impl LifeCycle {
    // TODO - link this to documentation of stashed widgets
    /// Whether this event should be sent to widgets which are currently not visible and not
    /// accessible.
    ///
    /// If a widget changes which children are `hidden` it must call [`children_changed`].
    /// For a more detailed explanation of the `hidden` state, see [`Event::should_propagate_to_hidden`].
    ///
    /// [`children_changed`]: crate::EventCtx::children_changed
    /// [`Event::should_propagate_to_hidden`]: Event::should_propagate_to_hidden
    pub fn should_propagate_to_hidden(&self) -> bool {
        match self {
            LifeCycle::Internal(internal) => internal.should_propagate_to_hidden(),
            LifeCycle::WidgetAdded => true,
            LifeCycle::DisabledChanged(_) => true,
            LifeCycle::BuildFocusChain => false,
            LifeCycle::RequestPanToChild(_) => false,
        }
    }

    /// Short name, for debug logging.
    ///
    /// Essentially returns the enum variant name.
    pub fn short_name(&self) -> &str {
        match self {
            LifeCycle::Internal(internal) => match internal {
                InternalLifeCycle::RouteWidgetAdded => "RouteWidgetAdded",
                InternalLifeCycle::RouteFocusChanged { .. } => "RouteFocusChanged",
                InternalLifeCycle::RouteDisabledChanged => "RouteDisabledChanged",
                InternalLifeCycle::ParentWindowOrigin => "ParentWindowOrigin",
            },
            LifeCycle::WidgetAdded => "WidgetAdded",
            LifeCycle::DisabledChanged(_) => "DisabledChanged",
            LifeCycle::BuildFocusChain => "BuildFocusChain",
            LifeCycle::RequestPanToChild(_) => "RequestPanToChild",
        }
    }
}

impl InternalLifeCycle {
    /// Whether this event should be sent to widgets which are currently not visible and not
    /// accessible.
    ///
    /// If a widget changes which children are `hidden` it must call [`children_changed`].
    /// For a more detailed explanation of the `hidden` state, see [`Event::should_propagate_to_hidden`].
    ///
    /// [`children_changed`]: crate::EventCtx::children_changed
    /// [`Event::should_propagate_to_hidden`]: Event::should_propagate_to_hidden
    pub fn should_propagate_to_hidden(&self) -> bool {
        match self {
            InternalLifeCycle::RouteWidgetAdded
            | InternalLifeCycle::RouteFocusChanged { .. }
            | InternalLifeCycle::RouteDisabledChanged => true,
            InternalLifeCycle::ParentWindowOrigin => false,
        }
    }
}
