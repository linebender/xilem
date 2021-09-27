// TODO - remove methods, add public fields

use crate::kurbo::{Point, Size};
use crate::platform::win_handler::AppState;
use crate::widget::LabelText;
use crate::Widget;

use druid_shell::WindowState;
use druid_shell::{Counter, Error as PlatformError, WindowBuilder, WindowHandle, WindowLevel};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

/// A description of a window to be instantiated.
pub struct WindowDesc {
    pub(crate) pending: PendingWindow,
    pub(crate) config: WindowConfig,
    /// The `WindowId` that will be assigned to this window.
    ///
    /// This can be used to track a window from when it is launched and when
    /// it actually connects.
    pub id: WindowId,
}

/// Defines how a windows size should be determined
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindowSizePolicy {
    /// Use the content of the window to determine the size.
    ///
    /// If you use this option, your root widget will be passed infinite constraints;
    /// you are responsible for ensuring that your content picks an appropriate size.
    Content,
    /// Use the provided window size
    User,
}

/// Window configuration that can be applied to a WindowBuilder, or to an existing WindowHandle.
/// It does not include anything related to app data.
#[derive(Default, Debug, PartialEq)]
pub struct WindowConfig {
    pub(crate) size_policy: WindowSizePolicy,
    pub(crate) size: Option<Size>,
    pub(crate) min_size: Option<Size>,
    pub(crate) position: Option<Point>,
    pub(crate) resizable: Option<bool>,
    pub(crate) transparent: Option<bool>,
    pub(crate) show_titlebar: Option<bool>,
    pub(crate) level: Option<WindowLevel>,
    pub(crate) state: Option<WindowState>,
}

/// The parts of a window, pending construction, that are dependent on top level app state
/// or are not part of the druid shells windowing abstraction.
/// This includes the boxed root widget, as well as other window properties such as the title.
pub struct PendingWindow {
    pub(crate) root: Box<dyn Widget>,
    pub(crate) title: LabelText,
    pub(crate) transparent: bool,
    pub(crate) size_policy: WindowSizePolicy, // This is copied over from the WindowConfig
                                              // when the native window is constructed.
}

// ---

impl WindowId {
    /// Allocate a new, unique window id.
    pub fn next() -> WindowId {
        static WINDOW_COUNTER: Counter = Counter::new();
        WindowId(WINDOW_COUNTER.next())
    }
}

impl WindowDesc {
    /// Create a new `WindowDesc`, taking the root [`Widget`] for this window.
    ///
    /// [`Widget`]: trait.Widget.html
    pub fn new<W>(root: W) -> WindowDesc
    where
        W: Widget + 'static,
    {
        WindowDesc {
            pending: PendingWindow::new(root),
            config: WindowConfig::default(),
            id: WindowId::next(),
        }
    }

    /// Set the title for this window. This is a [`LabelText`]; it can be either
    /// a `String`, a [`LocalizedString`], or a closure that computes a string;
    /// it will be kept up to date as the application's state changes.
    ///
    /// [`LabelText`]: widget/enum.LocalizedString.html
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: impl Into<LabelText>) -> Self {
        self.pending = self.pending.title(title);
        self
    }

    /// Set the window size policy
    pub fn window_size_policy(mut self, size_policy: WindowSizePolicy) -> Self {
        #[cfg(windows)]
        {
            // On Windows content_insets doesn't work on window with no initial size
            // so the window size can't be adapted to the content, to fix this a
            // non null initial size is set here.
            if size_policy == WindowSizePolicy::Content {
                self.config.size = Some(Size::new(1., 1.))
            }
        }
        self.config.size_policy = size_policy;
        self
    }

    /// Set the window's initial drawing area size in [display points].
    ///
    /// You can pass in a tuple `(width, height)` or a [`Size`],
    /// e.g. to create a window with a drawing area 1000dp wide and 500dp high:
    ///
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// [`Size`]: struct.Size.html
    /// [display points]: struct.Scale.html
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.config.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points].
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`].
    ///
    /// [`window_size`]: #method.window_size
    /// [display points]: struct.Scale.html
    pub fn with_min_size(mut self, size: impl Into<Size>) -> Self {
        self.config = self.config.with_min_size(size);
        self
    }

    /// Builder-style method to set whether this window can be resized.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config = self.config.resizable(resizable);
        self
    }

    /// Builder-style method to set whether this window's titlebar is visible.
    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.config = self.config.show_titlebar(show_titlebar);
        self
    }

    /// Builder-style method to set whether this window's background should be
    /// transparent.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.config = self.config.transparent(transparent);
        self.pending = self.pending.transparent(transparent);
        self
    }

    /// Sets the initial window position in [display points], relative to the origin
    /// of the [virtual screen].
    ///
    /// [display points]: crate::Scale
    /// [virtual screen]: crate::Screen
    pub fn set_position(mut self, position: impl Into<Point>) -> Self {
        self.config = self.config.set_position(position.into());
        self
    }

    /// Sets the [`WindowLevel`] of the window
    ///
    /// [`WindowLevel`]: enum.WindowLevel.html
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.config = self.config.set_level(level);
        self
    }

    /// Set initial state for the window.
    pub fn set_window_state(mut self, state: WindowState) -> Self {
        self.config = self.config.set_window_state(state);
        self
    }

    /// Set the [`WindowConfig`] of window.
    pub fn with_config(mut self, config: WindowConfig) -> Self {
        self.config = config;
        self
    }

    /// Attempt to create a platform window from this `WindowDesc`.
    pub(crate) fn build_native(self, state: &mut AppState) -> Result<WindowHandle, PlatformError> {
        state.build_native_window(self.id, self.pending, self.config)
    }
}

impl PendingWindow {
    /// Create a pending window from any widget.
    pub fn new<W>(root: W) -> PendingWindow
    where
        W: Widget + 'static,
    {
        // This just makes our API slightly cleaner; callers don't need to explicitly box.
        PendingWindow {
            root: Box::new(root),
            title: "widget_cruncher app".into(),
            transparent: false,
            size_policy: WindowSizePolicy::User,
        }
    }

    /// Set the title for this window. This is a [`LabelText`]; it can be either
    /// a `String`, a [`LocalizedString`], or a closure that computes a string;
    /// it will be kept up to date as the application's state changes.
    ///
    /// [`LabelText`]: widget/enum.LocalizedString.html
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: impl Into<LabelText>) -> Self {
        self.title = title.into();
        self
    }

    /// Set wether the background should be transparent
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
}

impl Default for WindowSizePolicy {
    fn default() -> Self {
        WindowSizePolicy::User
    }
}

impl WindowConfig {
    /// Set the window size policy.
    pub fn window_size_policy(mut self, size_policy: WindowSizePolicy) -> Self {
        #[cfg(windows)]
        {
            // On Windows content_insets doesn't work on window with no initial size
            // so the window size can't be adapted to the content, to fix this a
            // non null initial size is set here.
            if size_policy == WindowSizePolicy::Content {
                self.size = Some(Size::new(1., 1.))
            }
        }
        self.size_policy = size_policy;
        self
    }

    /// Set the window's initial drawing area size in [display points].
    ///
    /// You can pass in a tuple `(width, height)` or a [`Size`],
    /// e.g. to create a window with a drawing area 1000dp wide and 500dp high:
    ///
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// [`Size`]: struct.Size.html
    /// [display points]: struct.Scale.html
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points].
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`].
    ///
    /// [`window_size`]: #method.window_size
    /// [display points]: struct.Scale.html
    pub fn with_min_size(mut self, size: impl Into<Size>) -> Self {
        self.min_size = Some(size.into());
        self
    }

    /// Set whether the window should be resizable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    /// Set whether the window should have a titlebar and decorations.
    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.show_titlebar = Some(show_titlebar);
        self
    }

    /// Sets the window position in virtual screen coordinates.
    /// [`position`] Position in pixels.
    ///
    /// [`position`]: struct.Point.html
    pub fn set_position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the [`WindowLevel`] of the window
    ///
    /// [`WindowLevel`]: enum.WindowLevel.html
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Sets the [`WindowState`] of the window.
    ///
    /// [`WindowState`]: enum.WindowState.html
    pub fn set_window_state(mut self, state: WindowState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set whether the window background should be transparent
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// Apply this window configuration to the passed in WindowBuilder
    pub fn apply_to_builder(&self, builder: &mut WindowBuilder) {
        if let Some(resizable) = self.resizable {
            builder.resizable(resizable);
        }

        if let Some(show_titlebar) = self.show_titlebar {
            builder.show_titlebar(show_titlebar);
        }

        if let Some(size) = self.size {
            builder.set_size(size);
        } else if let WindowSizePolicy::Content = self.size_policy {
            builder.set_size(Size::new(0., 0.));
        }

        if let Some(position) = self.position {
            builder.set_position(position);
        }

        if let Some(transparent) = self.transparent {
            builder.set_transparent(transparent);
        }

        if let Some(level) = self.level {
            builder.set_level(level)
        }

        if let Some(state) = self.state {
            builder.set_window_state(state);
        }

        if let Some(min_size) = self.min_size {
            builder.set_min_size(min_size);
        }
    }

    /// Apply this window configuration to the passed in WindowHandle
    pub fn apply_to_handle(&self, win_handle: &mut WindowHandle) {
        if let Some(resizable) = self.resizable {
            win_handle.resizable(resizable);
        }

        if let Some(show_titlebar) = self.show_titlebar {
            win_handle.show_titlebar(show_titlebar);
        }

        if let Some(size) = self.size {
            win_handle.set_size(size);
        }

        // Can't apply min size currently as window handle
        // does not support it.

        if let Some(position) = self.position {
            win_handle.set_position(position);
        }

        if let Some(level) = self.level {
            win_handle.set_level(level)
        }

        if let Some(state) = self.state {
            win_handle.set_window_state(state);
        }
    }
}
