// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Events.

use crate::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
use crate::kurbo::Rect;
// TODO - See issue https://github.com/linebender/xilem/issues/367
use crate::WidgetId;

use std::path::PathBuf;

use winit::event::{Force, Ime, KeyEvent, Modifiers};
use winit::keyboard::ModifiersState;

// TODO - Occluded(bool) event
// TODO - winit ActivationTokenDone thing
// TODO - Suspended/Resume/NewEvents/MemoryWarning
// TODO - wtf is InnerSizeWriter?
// TODO - Move AnimFrame to Lifecycle
// TODO - switch anim frames to being about age / an absolute timestamp
// instead of time elapsed.
// (this will help in cases where we want to skip anim frames)
#[derive(Debug, Clone)]
pub enum WindowEvent {
    Rescale(f64),
    Resize(PhysicalSize<u32>),
    AnimFrame,
    RebuildAccessTree,
}

/// An indicator of which pointer button was pressed.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
#[repr(u8)]
pub enum PointerButton {
    /// No mouse button.
    None,
    /// Primary button, commonly the left mouse button, touch contact, pen contact.
    Primary,
    /// Secondary button, commonly the right mouse button, pen barrel button.
    Secondary,
    /// Auxiliary button, commonly the middle mouse button.
    Auxiliary,
    /// X1 (back) Mouse.
    X1,
    /// X2 (forward) Mouse.
    X2,
    /// Other mouse button. This isn't fleshed out yet.
    Other,
}

/// A set of [`PointerButton`]s.
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct PointerButtons(u8);

fn button_bit(button: PointerButton) -> u8 {
    match button {
        PointerButton::None => 0,
        PointerButton::Primary => 0b1,
        PointerButton::Secondary => 0b10,
        PointerButton::Auxiliary => 0b100,
        PointerButton::X1 => 0b1000,
        PointerButton::X2 => 0b10000,
        // TODO: When we properly do `Other`, this changes
        PointerButton::Other => 0b100000,
    }
}

impl PointerButtons {
    /// Create a new empty set.
    #[inline]
    pub fn new() -> PointerButtons {
        PointerButtons(0)
    }

    /// Add the `button` to the set.
    #[inline]
    pub fn insert(&mut self, button: PointerButton) {
        self.0 |= button_bit(button);
    }

    /// Remove the `button` from the set.
    #[inline]
    pub fn remove(&mut self, button: PointerButton) {
        self.0 &= !button_bit(button);
    }

    /// Returns `true` if the `button` is in the set.
    #[inline]
    pub fn contains(self, button: PointerButton) -> bool {
        (self.0 & button_bit(button)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all the `buttons` are in the set.
    #[inline]
    pub fn contains_all(self, buttons: PointerButtons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Adds all the `buttons` to the set.
    pub fn extend(&mut self, buttons: PointerButtons) {
        self.0 |= buttons.0;
    }

    /// Clear the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Count the number of pressed buttons in the set.
    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
}

impl std::fmt::Debug for PointerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut tuple = f.debug_tuple("PointerButtons");
        if self.contains(PointerButton::Primary) {
            tuple.field(&"Primary");
        }
        if self.contains(PointerButton::Secondary) {
            tuple.field(&"Secondary");
        }
        if self.contains(PointerButton::Auxiliary) {
            tuple.field(&"Auxiliary");
        }
        if self.contains(PointerButton::X1) {
            tuple.field(&"X1");
        }
        if self.contains(PointerButton::X2) {
            tuple.field(&"X2");
        }
        if self.contains(PointerButton::Other) {
            tuple.field(&"Other");
        }
        tuple.finish()
    }
}

impl std::fmt::Binary for PointerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

impl std::ops::BitOr for PointerButton {
    type Output = PointerButtons;

    fn bitor(self, rhs: Self) -> Self::Output {
        PointerButtons(button_bit(self) | button_bit(rhs))
    }
}

impl std::ops::BitOr<PointerButton> for PointerButtons {
    type Output = Self;

    fn bitor(self, rhs: PointerButton) -> Self {
        Self(self.0 | button_bit(rhs))
    }
}

impl std::ops::BitOrAssign<PointerButton> for PointerButtons {
    fn bitor_assign(&mut self, rhs: PointerButton) {
        self.0 |= button_bit(rhs);
    }
}

impl From<PointerButton> for PointerButtons {
    fn from(button: PointerButton) -> Self {
        Self(button_bit(button))
    }
}

// TODO - How can RenderRoot express "I started a drag-and-drop op"?
// TODO - Touchpad, Touch, AxisMotion
// TODO - How to handle CursorEntered?
// Note to self: Events like "pointerenter", "pointerleave" are handled differently at the Widget level. But that's weird because WidgetPod can distribute them. Need to think about this again.
#[derive(Debug, Clone)]
pub enum PointerEvent {
    PointerDown(PointerButton, PointerState),
    PointerUp(PointerButton, PointerState),
    PointerMove(PointerState),
    PointerEnter(PointerState),
    PointerLeave(PointerState),
    MouseWheel(LogicalPosition<f64>, PointerState),
    HoverFile(PathBuf, PointerState),
    DropFile(PathBuf, PointerState),
    HoverFileCancel(PointerState),
    Pinch(f64, PointerState),
}

// TODO - Clipboard Paste?
// TODO skip is_synthetic=true events
#[derive(Debug, Clone)]
pub enum TextEvent {
    KeyboardKey(KeyEvent, ModifiersState),
    Ime(Ime),
    ModifierChange(ModifiersState),
    // TODO - Document difference with Lifecycle focus change
    FocusChange(bool),
}

#[derive(Debug, Clone)]
pub struct AccessEvent {
    // TODO - Split out widget id from AccessEvent
    pub target: WidgetId,
    pub action: accesskit::Action,
    pub data: Option<accesskit::ActionData>,
}

#[derive(Debug, Clone)]
pub struct PointerState {
    // TODO
    // pub device_id: DeviceId,
    pub physical_position: PhysicalPosition<f64>,
    pub position: LogicalPosition<f64>,
    pub buttons: PointerButtons,
    pub mods: Modifiers,
    pub count: u8,
    pub focus: bool,
    pub force: Option<Force>,
}

#[derive(Debug, Clone)]
pub enum WindowTheme {
    Light,
    Dark,
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
    /// [`WidgetPod`]: crate::WidgetPod
    Internal(InternalLifeCycle),
}

/// Internal lifecycle events used by Masonry inside [`WidgetPod`].
///
/// These events are translated into regular [`LifeCycle`] events
/// and should not be used directly.
///
/// [`WidgetPod`]: crate::WidgetPod
#[derive(Debug, Clone)]
pub enum InternalLifeCycle {
    /// Used to route the `WidgetAdded` event to the required widgets.
    RouteWidgetAdded,

    /// Used to route the `DisabledChanged` event to the required widgets.
    RouteDisabledChanged,
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
    /// See [`is_hot`](crate::EventCtx::is_hot) for
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
    /// [`EventCtx::is_focused`]: crate::EventCtx::is_focused
    FocusChanged(bool),

    /// Called when a widget becomes or no longer is parent of a focused widget.
    HasFocusChanged(bool),
}

impl PointerEvent {
    pub fn pointer_state(&self) -> &PointerState {
        match self {
            PointerEvent::PointerDown(_, state)
            | PointerEvent::PointerUp(_, state)
            | PointerEvent::PointerMove(state)
            | PointerEvent::PointerEnter(state)
            | PointerEvent::PointerLeave(state)
            | PointerEvent::MouseWheel(_, state)
            | PointerEvent::HoverFile(_, state)
            | PointerEvent::DropFile(_, state)
            | PointerEvent::HoverFileCancel(state)
            | PointerEvent::Pinch(_, state) => state,
        }
    }

    pub fn position(&self) -> Option<LogicalPosition<f64>> {
        match self {
            PointerEvent::PointerLeave(_) | PointerEvent::HoverFileCancel(_) => None,
            _ => Some(self.pointer_state().position),
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            PointerEvent::PointerDown(_, _) => "PointerDown",
            PointerEvent::PointerUp(_, _) => "PointerUp",
            PointerEvent::PointerMove(_) => "PointerMove",
            PointerEvent::PointerEnter(_) => "PointerEnter",
            PointerEvent::PointerLeave(_) => "PointerLeave",
            PointerEvent::MouseWheel(_, _) => "MouseWheel",
            PointerEvent::HoverFile(_, _) => "HoverFile",
            PointerEvent::DropFile(_, _) => "DropFile",
            PointerEvent::HoverFileCancel(_) => "HoverFileCancel",
            PointerEvent::Pinch(_, _) => "Pinch",
        }
    }

    pub fn is_high_density(&self) -> bool {
        match self {
            PointerEvent::PointerDown(_, _) => false,
            PointerEvent::PointerUp(_, _) => false,
            PointerEvent::PointerMove(_) => true,
            PointerEvent::PointerEnter(_) => false,
            PointerEvent::PointerLeave(_) => false,
            PointerEvent::MouseWheel(_, _) => true,
            PointerEvent::HoverFile(_, _) => true,
            PointerEvent::DropFile(_, _) => false,
            PointerEvent::HoverFileCancel(_) => false,
            PointerEvent::Pinch(_, _) => true,
        }
    }
}

impl TextEvent {
    pub fn short_name(&self) -> &'static str {
        match self {
            TextEvent::KeyboardKey(_, _) => "KeyboardKey",
            TextEvent::Ime(Ime::Disabled) => "Ime::Disabled",
            TextEvent::Ime(Ime::Enabled) => "Ime::Enabled",
            TextEvent::Ime(Ime::Commit(_)) => "Ime::Commit",
            TextEvent::Ime(Ime::Preedit(s, _)) if s.is_empty() => "Ime::Preedit(\"\")",
            TextEvent::Ime(Ime::Preedit(_, _)) => "Ime::Preedit",
            TextEvent::ModifierChange(_) => "ModifierChange",
            TextEvent::FocusChange(_) => "FocusChange",
        }
    }

    pub fn is_high_density(&self) -> bool {
        match self {
            TextEvent::KeyboardKey(event, _) => event.repeat,
            TextEvent::Ime(_) => false,
            // Basically every mouse click/scroll event seems to produce a modifier change event.
            TextEvent::ModifierChange(_) => true,
            TextEvent::FocusChange(_) => false,
        }
    }
}

impl AccessEvent {
    pub fn short_name(&self) -> &'static str {
        match self.action {
            accesskit::Action::Default => "Default",
            accesskit::Action::Focus => "Focus",
            accesskit::Action::Blur => "Blur",
            accesskit::Action::Collapse => "Collapse",
            accesskit::Action::Expand => "Expand",
            accesskit::Action::CustomAction => "CustomAction",
            accesskit::Action::Decrement => "Decrement",
            accesskit::Action::Increment => "Increment",
            accesskit::Action::HideTooltip => "HideTooltip",
            accesskit::Action::ShowTooltip => "ShowTooltip",
            accesskit::Action::ReplaceSelectedText => "ReplaceSelectedText",
            accesskit::Action::ScrollBackward => "ScrollBackward",
            accesskit::Action::ScrollDown => "ScrollDown",
            accesskit::Action::ScrollForward => "ScrollForward",
            accesskit::Action::ScrollLeft => "ScrollLeft",
            accesskit::Action::ScrollRight => "ScrollRight",
            accesskit::Action::ScrollUp => "ScrollUp",
            accesskit::Action::ScrollIntoView => "ScrollIntoView",
            accesskit::Action::ScrollToPoint => "ScrollToPoint",
            accesskit::Action::SetScrollOffset => "SetScrollOffset",
            accesskit::Action::SetTextSelection => "SetTextSelection",
            accesskit::Action::SetSequentialFocusNavigationStartingPoint => {
                "SetSequentialFocusNavigationStartingPoint"
            }
            accesskit::Action::SetValue => "SetValue",
            accesskit::Action::ShowContextMenu => "ShowContextMenu",
        }
    }
}

impl PointerState {
    pub fn empty() -> Self {
        #[cfg(FALSE)]
        #[allow(unsafe_code)]
        // SAFETY: Uuuuh, unclear. Winit says the dummy id should only be used in
        // tests and should never be passed to winit. In principle, we're never
        // passing this id to winit, but it's still visible to custom widgets which
        // might do so if they tried really hard.
        // It would be a lot better if winit could just make this constructor safe.
        let device_id = unsafe { DeviceId::dummy() };

        PointerState {
            physical_position: PhysicalPosition::new(0.0, 0.0),
            position: LogicalPosition::new(0.0, 0.0),
            buttons: Default::default(),
            mods: Default::default(),
            count: 0,
            focus: false,
            force: None,
        }
    }
}

impl LifeCycle {
    // TODO - link this to documentation of stashed widgets - See issue https://github.com/linebender/xilem/issues/372
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
            LifeCycle::AnimFrame(_) => true,
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
                InternalLifeCycle::RouteDisabledChanged => "RouteDisabledChanged",
            },
            LifeCycle::WidgetAdded => "WidgetAdded",
            LifeCycle::AnimFrame(_) => "AnimFrame",
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
            InternalLifeCycle::RouteWidgetAdded | InternalLifeCycle::RouteDisabledChanged => true,
        }
    }
}
