// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Events.

use std::path::PathBuf;

use keyboard_types::{KeyboardEvent, Modifiers};
use vello::kurbo::Point;

use crate::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
use crate::kurbo::Rect;

// TODO - Occluded(bool) event
// TODO - winit ActivationTokenDone thing
// TODO - Suspended/Resume/NewEvents/MemoryWarning

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
    KeyboardKey(KeyboardEvent, Modifiers, Option<String>),
    /// An IME event.
    Ime(Ime),
    /// Modifier keys (e.g. Shift, Ctrl, Alt) were pressed or released.
    ModifierChange(Modifiers),
    /// The window took or lost focus.
    WindowFocusChange(bool),
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
    /// Container widgets (widgets which use [`WidgetPod`](crate::core::WidgetPod) to
    /// manage children) must ensure that this event is forwarded to those children.
    /// The [`WidgetPod`](crate::core::WidgetPod) itself will handle registering those
    /// children with the system; this is required for things like correct routing
    /// of events.
    WidgetAdded,

    /// Called when the Disabled state of the widget is changed.
    ///
    /// To check if a widget is disabled, see [`is_disabled`].
    ///
    /// To change a widget's disabled state, see [`set_disabled`].
    ///
    /// [`is_disabled`]: crate::core::EventCtx::is_disabled
    /// [`set_disabled`]: crate::core::EventCtx::set_disabled
    DisabledChanged(bool),

    // TODO - Link to tutorial doc.
    /// Called when the Stashed state of the widget is changed.
    ///
    /// To check if a widget is stashed, see [`is_stashed`].
    ///
    /// To change a widget's stashed state, see [`set_stashed`].
    ///
    /// [`is_stashed`]: crate::core::EventCtx::is_stashed
    /// [`set_stashed`]: crate::core::EventCtx::set_stashed
    StashedChanged(bool),

    /// Called when a child widgets uses
    /// [`EventCtx::request_scroll_to_this`](crate::core::EventCtx::request_scroll_to_this).
    RequestPanToChild(Rect),

    /// Called when the [hovered] status of the current widget changes.
    ///
    /// [hovered]: crate::doc::doc_06_masonry_concepts#widget-status
    HoveredChanged(bool),

    /// Called when the [hovered] status of the current widget or a descendant changes.
    ///
    /// This is sent before [`Update::HoveredChanged`].
    ///
    /// [hovered]: crate::doc::doc_06_masonry_concepts#widget-status
    ChildHoveredChanged(bool),

    /// Called when the [focused] status of the current widget changes.
    ///
    /// [focused]: crate::doc::doc_06_masonry_concepts#text-focus
    FocusChanged(bool),

    /// Called when the [focused] status of the current widget or a descendant changes.
    ///
    /// This is sent before [`Update::FocusChanged`].
    ///
    /// [focused]: crate::doc::doc_06_masonry_concepts#text-focus
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

    /// Returns `true` if the event is likely to occur every frame.
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
    /// Returns the position of this event in local (the widget's) coordinate space.
    pub fn local_position(&self, ctx: &crate::core::EventCtx) -> Point {
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
            physical_position: PhysicalPosition::default(),
            position: LogicalPosition::default(),
            buttons: PointerButtons::default(),
            mods: Modifiers::default(),
            count: 0,
            force: None,
        };
        Self::PointerLeave(pointer_state)
    }
}

impl TextEvent {
    /// Constructor for IME Preedit events.
    ///
    /// This is mostly useful for testing.
    pub fn preedit(text: String) -> Self {
        Self::Ime(Ime::Preedit(text, None))
    }

    /// Constructor for IME Preedit events.
    ///
    /// This is mostly useful for testing.
    ///
    /// **selected** is the part of the preedit text that should be selected.
    ///
    /// ## Panics
    ///
    /// If **selected** isn't a substring of **text**.
    pub fn preedit_with_cursor(text: String, selected: String) -> Self {
        let Some(offset) = text.find(&selected) else {
            panic!("Error building Preedit event: '{selected}' not found in '{text}'");
        };
        let span = (offset, selected.len());
        Self::Ime(Ime::Preedit(text, Some(span)))
    }

    /// Short name, for debug logging.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::KeyboardKey(KeyboardEvent { repeat: true, .. }, ..) => "KeyboardKey(repeat)",
            Self::KeyboardKey(..) => "KeyboardKey",
            Self::Ime(Ime::Disabled) => "Ime::Disabled",
            Self::Ime(Ime::Enabled) => "Ime::Enabled",
            Self::Ime(Ime::Commit(_)) => "Ime::Commit",
            Self::Ime(Ime::Preedit(s, _)) if s.is_empty() => "Ime::Preedit(\"\")",
            Self::Ime(Ime::Preedit(_, _)) => "Ime::Preedit(\"...\")",
            Self::ModifierChange(_) => "ModifierChange",
            Self::WindowFocusChange(_) => "WindowFocusChange",
        }
    }

    /// Returns `true` if the event is likely to occur every frame.
    ///
    /// Developers should avoid logging during high-density events to avoid
    /// cluttering the console.
    pub fn is_high_density(&self) -> bool {
        match self {
            Self::KeyboardKey(..) => false,
            Self::Ime(_) => false,
            // Basically every mouse click/scroll event seems to produce a modifier change event.
            Self::ModifierChange(_) => true,
            Self::WindowFocusChange(_) => false,
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
            buttons: PointerButtons::default(),
            mods: Modifiers::default(),
            count: 0,
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
            Self::ChildHoveredChanged(_) => "ChildHoveredChanged",
            Self::FocusChanged(_) => "FocusChanged",
            Self::ChildFocusChanged(_) => "ChildFocusChanged",
        }
    }
}

/// Describes [input method](https://en.wikipedia.org/wiki/Input_method) events.
///
/// Mirrors [`winit::event::Ime`].
///
/// This is also called a "composition event".
///
/// Most keypresses using a latin-like keyboard layout simply generate a
/// [`winit::event::WindowEvent::KeyboardInput`]. However, one couldn't possibly have a key for every single
/// unicode character that the user might want to type
/// - so the solution operating systems employ is to allow the user to type these using _a sequence
///   of keypresses_ instead.
///
/// A prominent example of this is accents - many keyboard layouts allow you to first click the
/// "accent key", and then the character you want to apply the accent to. In this case, some
/// platforms will generate the following event sequence:
///
/// ```ignore
/// // Press "`" key
/// Ime::Preedit("`", Some((0, 0)))
/// // Press "E" key
/// Ime::Preedit("", None) // Synthetic event generated by winit to clear preedit.
/// Ime::Commit("é")
/// ```
///
/// Additionally, certain input devices are configured to display a candidate box that allow the
/// user to select the desired character interactively. (To properly position this box, you must use
/// [`winit::window::Window::set_ime_cursor_area`].)
///
/// An example of a keyboard layout which uses candidate boxes is pinyin. On a latin keyboard the
/// following event sequence could be obtained:
///
/// ```ignore
/// // Press "A" key
/// Ime::Preedit("a", Some((1, 1)))
/// // Press "B" key
/// Ime::Preedit("a b", Some((3, 3)))
/// // Press left arrow key
/// Ime::Preedit("a b", Some((1, 1)))
/// // Press space key
/// Ime::Preedit("啊b", Some((3, 3)))
/// // Press space key
/// Ime::Preedit("", None) // Synthetic event generated by winit to clear preedit.
/// Ime::Commit("啊不")
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
//#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Ime {
    /// Notifies when the IME was enabled.
    ///
    /// After getting this event you could receive [`Preedit`][Self::Preedit] and
    /// [`Commit`][Self::Commit] events. You should also start performing IME related requests
    /// like [`winit::window::Window::set_ime_cursor_area`].
    Enabled,

    /// Notifies when a new composing text should be set at the cursor position.
    ///
    /// The value represents a pair of the preedit string and the cursor begin position and end
    /// position. When it's `None`, the cursor should be hidden. When `String` is an empty string
    /// this indicates that preedit was cleared.
    ///
    /// The cursor position is byte-wise indexed.
    Preedit(String, Option<(usize, usize)>),

    /// Notifies when text should be inserted into the editor widget.
    ///
    /// Right before this event winit will send empty [`Self::Preedit`] event.
    Commit(String),

    /// Notifies when the IME was disabled.
    ///
    /// After receiving this event you won't get any more [`Preedit`][Self::Preedit] or
    /// [`Commit`][Self::Commit] events until the next [`Enabled`][Self::Enabled] event. You should
    /// also stop issuing IME related requests like [`winit::window::Window::set_ime_cursor_area`] and clear
    /// pending preedit text.
    Disabled,
}

/// Describes the force of a touch event
///
/// Mirrors [`winit::event::Force`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Force {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really really hard, or not hard at all, depending on the device.
    Normalized(f64),
}

impl Force {
    /// Returns the force normalized to the range between 0.0 and 1.0 inclusive.
    ///
    /// Instead of normalizing the force, you should prefer to handle
    /// [`Force::Calibrated`] so that the amount of force the user has to apply is
    /// consistent across devices.
    pub fn normalized(&self) -> f64 {
        match self {
            Self::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => {
                let force = match altitude_angle {
                    Some(altitude_angle) => force / altitude_angle.sin(),
                    None => *force,
                };
                force / max_possible_force
            }
            Self::Normalized(force) => *force,
        }
    }
}

/// Defines the orientation that a window resize will be performed.
///
/// Mirrors [`winit::window::ResizeDirection`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[expect(missing_docs, reason = "Copied from winit")]
pub enum ResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}
