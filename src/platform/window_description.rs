// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use druid_shell::{Counter, WindowBuilder, WindowHandle, WindowLevel, WindowState};

use crate::kurbo::{Point, Size};
use crate::{ArcStr, Widget};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

/// A description of a window to be instantiated.
///
/// This object is paramaterized with builder-style methods, eg:
///
/// ```no_run
/// # use masonry::WindowDescription;
/// # let some_widget = masonry::widget::Label::new("hello");
/// let main_window = WindowDescription::new(some_widget)
///     .title("My window")
///     .window_size((400.0, 400.0));
/// ```
pub struct WindowDescription {
    pub(crate) root: Box<dyn Widget>,
    pub(crate) title: ArcStr,
    pub(crate) config: WindowConfig,
    /// The `WindowId` that will be assigned to this window.
    ///
    /// This can be used to track a window from when it is launched to when
    /// it actually connects.
    pub id: WindowId,
}

/// Defines how a windows size should be determined
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum WindowSizePolicy {
    /// Use the content of the window to determine the size.
    ///
    /// If you use this option, your root widget will be passed infinite constraints;
    /// you are responsible for ensuring that your content picks an appropriate size.
    Content,
    /// Use the provided window size.
    #[default]
    User,
}

/// Window configuration that can be applied to a [WindowBuilder], or to an existing [WindowHandle].
///
/// It does not include anything related to app data.
#[derive(Default, PartialEq)]
pub struct WindowConfig {
    pub(crate) size_policy: WindowSizePolicy,
    pub(crate) size: Option<Size>,
    pub(crate) min_size: Option<Size>,
    pub(crate) position: Option<Point>,
    pub(crate) resizable: Option<bool>,
    pub(crate) transparent: Option<bool>,
    pub(crate) show_titlebar: Option<bool>,
    pub(crate) level: Option<WindowLevel>,
    // TODO - Remove?
    pub(crate) state: Option<WindowState>,
}

// ---

impl WindowId {
    /// Allocate a new, unique window id.
    pub fn next() -> WindowId {
        static WINDOW_COUNTER: Counter = Counter::new();
        WindowId(WINDOW_COUNTER.next())
    }
}

impl WindowDescription {
    /// Create a new `WindowDescription`, taking the root [`Widget`] for this window.
    pub fn new<W>(root: W) -> WindowDescription
    where
        W: Widget + 'static,
    {
        WindowDescription {
            root: Box::new(root),
            // FIXME - add argument instead
            title: "Masonry application".into(),
            config: WindowConfig::default(),
            id: WindowId::next(),
        }
    }

    /// Set the window title
    pub fn title(mut self, title: impl Into<ArcStr>) -> Self {
        self.title = title.into();
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

    /// Set the window's initial drawing area size in [display points](druid_shell::Scale).
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
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.config.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points](druid_shell::Scale).
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`](Self::window_size).
    pub fn min_size(mut self, size: impl Into<Size>) -> Self {
        self.config = self.config.min_size(size);
        self
    }

    /// Set whether this window can be resized.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config = self.config.resizable(resizable);
        self
    }

    /// Set whether this window's titlebar is visible.
    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.config = self.config.show_titlebar(show_titlebar);
        self
    }

    /// Set whether this window's background should be transparent.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.config = self.config.transparent(transparent);
        self
    }

    /// Set the initial window position in [display points](druid_shell::Scale), relative to the origin
    /// of the [virtual screen](druid_shell::Screen).
    pub fn set_position(mut self, position: impl Into<Point>) -> Self {
        self.config = self.config.set_position(position.into());
        self
    }

    /// Set the [`WindowLevel`] of the window.
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.config = self.config.set_level(level);
        self
    }

    /// Set initial [`WindowState`] of the window (eg minimized/maximized).
    pub fn set_window_state(mut self, state: WindowState) -> Self {
        self.config = self.config.set_window_state(state);
        self
    }

    /// Set the [`WindowConfig`] of the window.
    pub fn with_config(mut self, config: WindowConfig) -> Self {
        self.config = config;
        self
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

    /// Set the window's initial drawing area size in [display points](druid_shell::Scale).
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
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points](druid_shell::Scale).
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`](WindowConfig::window_size).
    pub fn min_size(mut self, size: impl Into<Size>) -> Self {
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

    /// Set the window position in virtual screen coordinates.
    ///
    /// Position is in pixels.
    pub fn set_position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    /// Set the [`WindowLevel`] of the window
    ///
    /// [`WindowLevel`]: enum.WindowLevel.html
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Set the [`WindowState`] of the window.
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

    /// Apply this window configuration to the given WindowBuilder
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

        if let Some(level) = &self.level {
            builder.set_level(level.clone())
        }

        if let Some(state) = self.state {
            builder.set_window_state(state);
        }

        if let Some(min_size) = self.min_size {
            builder.set_min_size(min_size);
        }
    }

    /// Apply this window configuration to the given WindowHandle
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

        // TODO - set_level ?
        // See https://github.com/linebender/druid/issues/1824

        if let Some(state) = self.state {
            win_handle.set_window_state(state);
        }
    }
}

impl std::fmt::Debug for WindowConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowConfig")
            .field("size_policy", &self.size_policy)
            .field("size", &self.size)
            .field("min_size", &self.min_size)
            .field("position", &self.position)
            .field("resizable", &self.resizable)
            .field("transparent", &self.transparent)
            .field("show_titlebar", &self.show_titlebar)
            .field(
                "level",
                match &self.level {
                    Some(WindowLevel::AppWindow) => &"Some(AppWindow)",
                    Some(WindowLevel::Tooltip(_)) => &"Some(ToolTip)",
                    Some(WindowLevel::DropDown(_)) => &"Some(DropDown)",
                    Some(WindowLevel::Modal(_)) => &"Some(Modal)",
                    None => &"None",
                },
            )
            .field("state", &self.state)
            .finish()
    }
}
