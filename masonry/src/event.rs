// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Events.

use std::path::PathBuf;

use vello::kurbo::Point;
use winit::event::{Force, Ime, KeyEvent, Modifiers};
use winit::keyboard::ModifiersState;

use crate::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
use crate::kurbo::Rect;

// TODO - Occluded(bool) event
// TODO - winit ActivationTokenDone thing
// TODO - Suspended/Resume/NewEvents/MemoryWarning
// TODO - wtf is InnerSizeWriter?
// TODO - Move AnimFrame to Update
// TODO - switch anim frames to being about age / an absolute timestamp
// instead of time elapsed.
// (this will help in cases where we want to skip anim frames)

/// A global event.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// The window's DPI factor changed.
    Rescale(f64),
    /// The window was resized.
    Resize(PhysicalSize<u32>),
    /// The animation frame requested by this window must run.
    AnimFrame,
    /// The accessibility tree must be rebuilt.
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
    pub fn new() -> Self {
        Self(0)
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
    pub fn contains_all(self, buttons: Self) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Adds all the `buttons` to the set.
    pub fn extend(&mut self, buttons: Self) {
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

// TODO - Document units for MouseWheel and Pinch deltas.
// TODO - How can RenderRoot express "I started a drag-and-drop op"?
// TODO - Touchpad, Touch, AxisMotion
// TODO - How to handle CursorEntered?
/// A pointer-related event.
///
/// A pointer in this context can be a mouse, a pen, a touch screen, etc. Though
/// Masonry currently doesn't really support multiple pointers.
#[derive(Debug, Clone)]
pub enum PointerEvent {
    /// A pointer was pressed.
    PointerDown(PointerButton, PointerState),
    /// A pointer was released.
    PointerUp(PointerButton, PointerState),
    /// A pointer was moved.
    PointerMove(PointerState),
    /// A pointer entered the window.
    PointerEnter(PointerState),
    /// A pointer left the window.
    ///
    /// A synthetic `PointerLeave` event may also be sent when a widget
    /// loses [pointer capture](crate::doc::doc_06_masonry_concepts#pointer-capture).
    PointerLeave(PointerState),
    /// A mouse wheel event.
    ///
    /// The first tuple value is the scrolled distances. In most cases with a
    /// standard mouse wheel, x will be 0 and y will be the number of ticks
    /// scrolled. Trackballs and touchpads may produce non-zero values for x.
    MouseWheel(LogicalPosition<f64>, PointerState),
    /// During a file drag-and-drop operation, a file was kept over the window.
    HoverFile(PathBuf, PointerState),
    /// During a file drag-and-drop operation, a file was dropped on the window.
    DropFile(PathBuf, PointerState),
    /// A file drag-and-drop operation was cancelled.
    HoverFileCancel(PointerState),
    /// A pinch gesture was detected.
    ///
    /// The first tuple value is the delta. Positive values indicate magnification
    /// (zooming in) and negative values indicate shrinking (zooming out).
    Pinch(f64, PointerState),
}

// TODO - Clipboard Paste?
// TODO skip is_synthetic=true events
/// A text-related event.
#[derive(Debug, Clone)]
pub enum TextEvent {
    /// A keyboard event.
    KeyboardKey(KeyEvent, ModifiersState),
    /// An IME event.
    Ime(Ime),
    /// Modifier keys (e.g. Shift, Ctrl, Alt) were pressed or released.
    ModifierChange(ModifiersState),
    /// The window took or lost focus.
    // TODO - Document difference with Update focus change
    FocusChange(bool),
}

// TODO - Go into more detail.
/// An accessibility event.
#[derive(Debug, Clone)]
pub struct AccessEvent {
    /// The action that was performed.
    pub action: accesskit::Action,
    /// The data associated with the action.
    pub data: Option<accesskit::ActionData>,
}

/// The persistent state of a pointer.
#[derive(Debug, Clone)]
pub struct PointerState {
    // TODO
    // pub device_id: DeviceId,
    /// The position of the pointer in physical coordinates.
    /// This is the number of pixels from the top and left of the window.
    pub physical_position: PhysicalPosition<f64>,

    /// The position of the pointer in logical coordinates.
    /// This is different from physical coordinates for high-DPI displays.
    pub position: LogicalPosition<f64>,

    /// The buttons that are currently pressed (mostly useful for the mouse).
    pub buttons: PointerButtons,

    /// The modifier keys (e.g. Shift, Ctrl, Alt) that are currently pressed.
    pub mods: Modifiers,

    /// The number of successive clicks registered. This is used to detect e.g. double-clicks.
    pub count: u8,

    // TODO - Find out why this was added, maybe remove it.
    /// Currently unused.
    pub focus: bool,

    /// The force of a touch event.
    pub force: Option<Force>,
}

/// The light/dark mode of the window.
#[derive(Debug, Clone)]
pub enum WindowTheme {
    /// Light mode.
    Light,
    /// Dark mode.
    Dark,
}

// TODO - Rewrite that doc.
/// Changes to widget state.
///
/// Unlike [`PointerEvent`]s, [`TextEvent`]s and [`AccessEvent`]s, [`Update`] events
/// are generated by Masonry, and may occur at different times during a given pass of
/// the event loop.
#[non_exhaustive]
#[derive(Debug, Clone)]
#[allow(variant_size_differences)]
pub enum Update {
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

    /// Called when the Disabled state of the widget is changed.
    ///
    /// To check if a widget is disabled, see [`is_disabled`].
    ///
    /// To change a widget's disabled state, see [`set_disabled`].
    ///
    /// [`is_disabled`]: crate::EventCtx::is_disabled
    /// [`set_disabled`]: crate::EventCtx::set_disabled
    DisabledChanged(bool),

    // TODO - Link to tutorial doc.
    /// Called when the Stashed state of the widget is changed.
    ///
    /// To check if a widget is stashed, see [`is_stashed`].
    ///
    /// To change a widget's stashed state, see [`set_stashed`].
    ///
    /// [`is_stashed`]: crate::EventCtx::is_stashed
    /// [`set_stashed`]: crate::EventCtx::set_stashed
    StashedChanged(bool),

    /// Called when a child widgets uses
    /// [`EventCtx::request_scroll_to_this`](crate::EventCtx::request_scroll_to_this).
    RequestPanToChild(Rect),

    /// Called when the "hovered" status changes.
    ///
    /// This will always be called _before_ the event that triggered it; that is,
    /// when the mouse moves over a widget, that widget will receive
    /// `Update::HoveredChanged` before it receives `Event::MouseMove`.
    ///
    /// See [`is_hovered`](crate::EventCtx::is_hovered) for
    /// discussion about the hovered status.
    HoveredChanged(bool),

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
    ChildFocusChanged(bool),
}

impl PointerEvent {
    /// Returns the [`PointerState`] of the event.
    pub fn pointer_state(&self) -> &PointerState {
        match self {
            Self::PointerDown(_, state)
            | Self::PointerUp(_, state)
            | Self::PointerMove(state)
            | Self::PointerEnter(state)
            | Self::PointerLeave(state)
            | Self::MouseWheel(_, state)
            | Self::HoverFile(_, state)
            | Self::DropFile(_, state)
            | Self::HoverFileCancel(state)
            | Self::Pinch(_, state) => state,
        }
    }

    /// Returns the position of the pointer event, except for [`PointerEvent::PointerLeave`] and [`PointerEvent::HoverFileCancel`].
    pub fn position(&self) -> Option<LogicalPosition<f64>> {
        match self {
            Self::PointerLeave(_) | Self::HoverFileCancel(_) => None,
            _ => Some(self.pointer_state().position),
        }
    }

    /// Short name, for debug logging.
    ///
    /// Returns the enum variant name.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::PointerDown(_, _) => "PointerDown",
            Self::PointerUp(_, _) => "PointerUp",
            Self::PointerMove(_) => "PointerMove",
            Self::PointerEnter(_) => "PointerEnter",
            Self::PointerLeave(_) => "PointerLeave",
            Self::MouseWheel(_, _) => "MouseWheel",
            Self::HoverFile(_, _) => "HoverFile",
            Self::DropFile(_, _) => "DropFile",
            Self::HoverFileCancel(_) => "HoverFileCancel",
            Self::Pinch(_, _) => "Pinch",
        }
    }

    /// Returns true if the event is likely to occur every frame.
    ///
    /// Developers should avoid logging during high-density events to avoid
    /// cluttering the console.
    pub fn is_high_density(&self) -> bool {
        match self {
            Self::PointerDown(_, _) => false,
            Self::PointerUp(_, _) => false,
            Self::PointerMove(_) => true,
            Self::PointerEnter(_) => false,
            Self::PointerLeave(_) => false,
            Self::MouseWheel(_, _) => true,
            Self::HoverFile(_, _) => true,
            Self::DropFile(_, _) => false,
            Self::HoverFileCancel(_) => false,
            Self::Pinch(_, _) => true,
        }
    }

    // TODO Logical/PhysicalPosition as return type instead?
    pub fn local_position(&self, ctx: &crate::EventCtx) -> Point {
        let position = self.pointer_state().position;
        ctx.widget_state.window_transform.inverse() * Point::new(position.x, position.y)
    }

    /// Create a [`PointerEvent::PointerLeave`] event with dummy values.
    ///
    /// This is used internally to create synthetic `PointerLeave` events when pointer
    /// capture is lost.
    pub fn new_pointer_leave() -> Self {
        // TODO - The fact we're creating so many dummy values might be
        // a sign we should refactor that struct
        let pointer_state = PointerState {
            physical_position: Default::default(),
            position: Default::default(),
            buttons: Default::default(),
            mods: Default::default(),
            count: 0,
            focus: false,
            force: None,
        };
        Self::PointerLeave(pointer_state)
    }
}

impl TextEvent {
    /// Short name, for debug logging.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::KeyboardKey(KeyEvent { repeat: true, .. }, _) => "KeyboardKey(repeat)",
            Self::KeyboardKey(_, _) => "KeyboardKey",
            Self::Ime(Ime::Disabled) => "Ime::Disabled",
            Self::Ime(Ime::Enabled) => "Ime::Enabled",
            Self::Ime(Ime::Commit(_)) => "Ime::Commit",
            Self::Ime(Ime::Preedit(s, _)) if s.is_empty() => "Ime::Preedit(\"\")",
            Self::Ime(Ime::Preedit(_, _)) => "Ime::Preedit(\"...\")",
            Self::ModifierChange(_) => "ModifierChange",
            Self::FocusChange(_) => "FocusChange",
        }
    }

    /// Returns true if the event is likely to occur every frame.
    ///
    /// Developers should avoid logging during high-density events to avoid
    /// cluttering the console.
    pub fn is_high_density(&self) -> bool {
        match self {
            Self::KeyboardKey(_, _) => false,
            Self::Ime(_) => false,
            // Basically every mouse click/scroll event seems to produce a modifier change event.
            Self::ModifierChange(_) => true,
            Self::FocusChange(_) => false,
        }
    }
}

impl AccessEvent {
    /// Short name, for debug logging.
    ///
    /// Returns the enum variant name.
    pub fn short_name(&self) -> &'static str {
        match self.action {
            accesskit::Action::Click => "Click",
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
    /// Create a new [`PointerState`] with dummy values.
    ///
    /// Mostly used for testing.
    pub fn empty() -> Self {
        Self {
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

impl Update {
    /// Short name, for debug logging.
    ///
    /// Returns the enum variant name.
    pub fn short_name(&self) -> &str {
        match self {
            Self::WidgetAdded => "WidgetAdded",
            Self::DisabledChanged(_) => "DisabledChanged",
            Self::StashedChanged(_) => "StashedChanged",
            Self::RequestPanToChild(_) => "RequestPanToChild",
            Self::HoveredChanged(_) => "HoveredChanged",
            Self::FocusChanged(_) => "FocusChanged",
            Self::ChildFocusChanged(_) => "ChildFocusChanged",
        }
    }
}
