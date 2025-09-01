// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::dpi::{Position, Size};
use winit::window::{Cursor, Icon, Window, WindowAttributes};

// TODO: make this a type-state builder to force Xilem::new apps to define on_close?
/// Attributes and callbacks of a window.
///
/// When passed to [`Xilem::new_simple`](crate::Xilem::new_simple) these are used to create the window.
/// When returned from the app logic function passed to [`Xilem::new`](crate::Xilem::new)
/// they can also be used to update the attributes/callbacks in the running application,
/// except if the attribute name starts with `initial_`. Attempting to change an initial-only attribute
/// won't have any effect and will result in a warning being logged.
pub struct WindowOptions<State> {
    pub(crate) reactive: ReactiveWindowAttrs,
    pub(crate) initial: InitialAttrs,
    pub(crate) callbacks: WindowCallbacks<State>,
}

/// These are attributes the user cannot change, so we can make them reactive.
#[derive(Clone, Debug)]
pub(crate) struct ReactiveWindowAttrs {
    title: String,
    resizable: bool,
    cursor: Cursor,
    min_inner_size: Option<Size>,
    max_inner_size: Option<Size>,
    platform_specific: PlatformSpecificReactiveWindowAttrs,
}

/// These are attributes the user can change, so we cannot make them reactive.
#[derive(Clone, Debug)]
pub(crate) struct InitialAttrs {
    inner_size: Option<Size>,
    position: Option<Position>,
    // TODO: move window_icon to ReactiveWindowAttrs once the winit type implements PartialEq
    window_icon: Option<Icon>,
    platform_specific: PlatformSpecificInitialWindowAttrs,
}

pub(crate) struct WindowCallbacks<State> {
    pub(crate) on_close: Option<Box<dyn Fn(&mut State)>>,
}
impl<S> Default for WindowCallbacks<S> {
    fn default() -> Self {
        Self { on_close: None }
    }
}

impl<State> WindowOptions<State> {
    /// Initializes a new window attributes builder with the given window title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            reactive: ReactiveWindowAttrs {
                title: title.into(),
                resizable: true,
                cursor: Cursor::default(),
                min_inner_size: None,
                max_inner_size: None,
                platform_specific: PlatformSpecificReactiveWindowAttrs::default(),
            },
            initial: InitialAttrs {
                inner_size: None,
                position: None,
                window_icon: None,
                platform_specific: PlatformSpecificInitialWindowAttrs::default(),
            },
            callbacks: WindowCallbacks::default(),
        }
    }

    /// Sets a callback to execute when the user has requested to close the window.
    pub fn on_close(mut self, callback: impl Fn(&mut State) + 'static) -> Self {
        self.callbacks.on_close = Some(Box::new(callback));
        self
    }

    /// Sets whether the window is resizable or not.
    ///
    /// The default is `true`.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.reactive.resizable = resizable;
        self
    }

    /// Sets the cursor icon of the window.
    pub fn with_cursor(mut self, cursor: impl Into<Cursor>) -> Self {
        self.reactive.cursor = cursor.into();
        self
    }

    /// Sets the minimum dimensions the window can have.
    pub fn with_min_inner_size<S: Into<Size>>(mut self, min_size: S) -> Self {
        self.reactive.min_inner_size = Some(min_size.into());
        self
    }

    /// Sets the maximum dimensions the window can have.
    pub fn with_max_inner_size<S: Into<Size>>(mut self, max_size: S) -> Self {
        self.reactive.max_inner_size = Some(max_size.into());
        self
    }

    /// Requests the window to be of specific dimensions.
    pub fn with_initial_inner_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.initial.inner_size = Some(size.into());
        self
    }

    /// Sets a desired initial position for the window.
    pub fn with_initial_position<P: Into<Position>>(mut self, position: P) -> Self {
        self.initial.position = Some(position.into());
        self
    }

    /// Sets the window icon.
    ///
    /// The default is `None`.
    pub fn with_initial_window_icon(mut self, window_icon: Option<Icon>) -> Self {
        self.initial.window_icon = window_icon;
        self
    }

    pub(crate) fn build_initial_attrs(&self) -> WindowAttributes {
        let mut attrs = WindowAttributes::default()
            .with_title(self.reactive.title.clone())
            .with_cursor(self.reactive.cursor.clone())
            .with_resizable(self.reactive.resizable)
            .with_window_icon(self.initial.window_icon.clone());

        if let Some(min_inner_size) = self.reactive.min_inner_size {
            attrs = attrs.with_min_inner_size(min_inner_size);
        }
        if let Some(max_inner_size) = self.reactive.max_inner_size {
            attrs = attrs.with_max_inner_size(max_inner_size);
        }
        if let Some(inner_size) = self.initial.inner_size {
            attrs = attrs.with_inner_size(inner_size);
        }
        self.initial
            .platform_specific
            .build(self.reactive.platform_specific.build(attrs))
    }

    pub(crate) fn rebuild(&self, prev: &Self, window: &Window) {
        self.rebuild_reactive_window_attributes(prev, window);
        self.warn_for_changed_initial_attributes(prev);
    }

    fn rebuild_reactive_window_attributes(&self, prev: &Self, window: &Window) {
        let current = &self.reactive;
        let prev = &prev.reactive;

        if current.title != prev.title {
            window.set_title(&current.title);
        }
        if current.resizable != prev.resizable {
            window.set_resizable(current.resizable);
        }
        if current.cursor != prev.cursor {
            window.set_cursor(current.cursor.clone());
        }
        if current.min_inner_size != prev.min_inner_size {
            window.set_min_inner_size(current.min_inner_size);
        }
        if current.max_inner_size != prev.max_inner_size {
            window.set_max_inner_size(current.max_inner_size);
        }

        current
            .platform_specific
            .rebuild(&prev.platform_specific, window);
    }

    fn warn_for_changed_initial_attributes(&self, prev: &Self) {
        let current = &self.initial;
        let prev = &prev.initial;

        if current.inner_size != prev.inner_size {
            tracing::warn!(
                "attempted to change inner_size attribute after window creation, this is not supported"
            );
        }
        if current.position != prev.position {
            tracing::warn!(
                "attempted to change position attribute after window creation, this is not supported"
            );
        }
        // winit::icon::Icon doesn't implement PartialEq, once it does it will be made reactive
        if current.window_icon.is_some() != prev.window_icon.is_some() {
            tracing::warn!(
                "attempted to change window_icon attribute after window creation, this is not supported"
            );
        }

        current.platform_specific.warn(&prev.platform_specific);
    }
}

#[cfg(windows)]
mod windows {
    #![expect(unsafe_code, reason = "FFI with Windows API")]

    use winit::{
        platform::windows::{
            BackdropType, Color, CornerPreference, HMENU, HWND, WindowAttributesExtWindows,
            WindowExtWindows,
        },
        window::{Icon, Window, WindowAttributes},
    };

    #[derive(Clone, Debug)]
    pub(crate) struct PlatformSpecificInitialWindowAttrs {
        owner: Option<HWND>,
        menu: Option<HMENU>,
        no_redirection_bitmap: bool,
        taskbar_icon: Option<Icon>,
        drag_and_drop: bool,
        class_name: String,
        clip_children: bool,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct PlatformSpecificReactiveWindowAttrs {
        skip_taskbar: bool,
        decoration_shadow: bool,
        backdrop_type: BackdropType,
        border_color: Option<Color>,
        title_background_color: Option<Color>,
        title_text_color: Option<Color>,
        corner_preference: Option<CornerPreference>,
    }

    impl PlatformSpecificInitialWindowAttrs {
        pub(crate) fn build(&self, attrs: WindowAttributes) -> WindowAttributes {
            let mut attrs = attrs
                .with_taskbar_icon(self.taskbar_icon.clone())
                .with_no_redirection_bitmap(self.no_redirection_bitmap)
                .with_drag_and_drop(self.drag_and_drop)
                .with_class_name(self.class_name.clone())
                .with_clip_children(self.clip_children);
            if let Some(owner) = self.owner {
                attrs = attrs.with_owner_window(owner);
            }
            if let Some(menu) = self.menu {
                attrs = attrs.with_menu(menu);
            }
            attrs
        }

        pub(crate) fn warn(&self, prev: &Self) {
            if self.owner != prev.owner {
                tracing::warn!(
                    "attempted to change owner attribute after window creation, this is not supported"
                );
            }
            if self.menu != prev.menu {
                tracing::warn!(
                    "attempted to change menu attribute after window creation, this is not supported"
                );
            }
            if self.no_redirection_bitmap != prev.no_redirection_bitmap {
                tracing::warn!(
                    "attempted to change no_redirection_bitmap attribute after window creation, this is not supported"
                );
            }
            if self.taskbar_icon.is_some() != prev.taskbar_icon.is_some() {
                tracing::warn!(
                    "attempted to change taskbar_icon attribute after window creation, this is not supported"
                );
            }
            if self.drag_and_drop != prev.drag_and_drop {
                tracing::warn!(
                    "attempted to change drag_and_drop attribute after window creation, this is not supported"
                );
            }
            if self.class_name != prev.class_name {
                tracing::warn!(
                    "attempted to change class_name attribute after window creation, this is not supported"
                );
            }
            if self.clip_children != prev.clip_children {
                tracing::warn!(
                    "attempted to change clip_children attribute after window creation, this is not supported"
                );
            }
        }
    }

    impl PlatformSpecificReactiveWindowAttrs {
        pub(crate) fn build(&self, attrs: WindowAttributes) -> WindowAttributes {
            let mut attrs = attrs
                .with_skip_taskbar(self.skip_taskbar)
                .with_undecorated_shadow(self.decoration_shadow)
                .with_system_backdrop(self.backdrop_type)
                .with_border_color(self.border_color)
                .with_title_background_color(self.title_background_color);
            if let Some(title_text_color) = self.title_text_color {
                attrs = attrs.with_title_text_color(title_text_color);
            }
            if let Some(corner_preference) = self.corner_preference {
                attrs = attrs.with_corner_preference(corner_preference);
            }
            attrs
        }

        pub(crate) fn rebuild(&self, prev: &Self, window: &Window) {
            // TODO: move taskbar_icon to ReactiveWindowAttrs once the winit type implements PartialEq
            // if self.taskbar_icon != prev.taskbar_icon {
            //     window.set_taskbar_icon(self.taskbar_icon)
            // }
            if self.skip_taskbar != prev.skip_taskbar {
                window.set_skip_taskbar(self.skip_taskbar);
            }
            if self.decoration_shadow != prev.decoration_shadow {
                window.set_undecorated_shadow(self.decoration_shadow);
            }
            if self.backdrop_type != prev.backdrop_type {
                window.set_system_backdrop(self.backdrop_type);
            }
            if self.border_color != prev.border_color {
                window.set_border_color(self.border_color);
            }
            if self.title_background_color != prev.title_background_color {
                window.set_title_background_color(self.title_background_color);
            }
            if self.title_text_color != prev.title_text_color
                && let Some(c) = self.title_text_color
            {
                window.set_title_text_color(c);
            }
            if self.corner_preference != prev.corner_preference
                && let Some(c) = self.corner_preference
            {
                window.set_corner_preference(c);
            }
        }
    }

    impl Default for PlatformSpecificInitialWindowAttrs {
        fn default() -> Self {
            Self {
                owner: None,
                menu: None,
                taskbar_icon: None,
                no_redirection_bitmap: false,
                drag_and_drop: true,
                class_name: "Window Class".to_string(),
                clip_children: true,
            }
        }
    }

    unsafe impl Send for PlatformSpecificReactiveWindowAttrs {}
    unsafe impl Sync for PlatformSpecificReactiveWindowAttrs {}

    /// Extension setters for Windows-specific window options.
    pub trait WindowOptionsExtWindows {
        /// Set an owner to the window to be created. Can be used to create a dialog box, for example.
        /// This only works when [`WindowAttributes::with_parent_window`] isn't called or set to `None`.
        /// Can be used in combination with
        /// [`WindowExtWindows::set_enable(false)`][WindowExtWindows::set_enable] on the owner
        /// window to create a modal dialog box.
        ///
        /// From MSDN:
        /// - An owned window is always above its owner in the z-order.
        /// - The system automatically destroys an owned window when its owner is destroyed.
        /// - An owned window is hidden when its owner is minimized.
        ///
        /// For more information, see <https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#owned-windows>
        fn with_owner_window(self, parent: HWND) -> Self;

        /// Sets a menu on the window to be created.
        ///
        /// Parent and menu are mutually exclusive; a child window cannot have a menu!
        ///
        /// The menu must have been manually created beforehand with `CreateMenu` or similar.
        ///
        /// Note: Dark mode cannot be supported for win32 menus, it's simply not possible to change how
        /// the menus look. If you use this, it is recommended that you combine it with
        /// `with_theme(Some(Theme::Light))` to avoid a jarring effect.
        fn with_menu(self, menu: HMENU) -> Self;

        /// This sets `ICON_BIG`. A good ceiling here is 256x256.
        fn with_taskbar_icon(self, taskbar_icon: Option<Icon>) -> Self;

        /// This sets `WS_EX_NOREDIRECTIONBITMAP`.
        fn with_no_redirection_bitmap(self, flag: bool) -> Self;

        /// Enables or disables drag and drop support (enabled by default). Will interfere with other
        /// crates that use multi-threaded COM API (`CoInitializeEx` with `COINIT_MULTITHREADED`
        /// instead of `COINIT_APARTMENTTHREADED`) on the same thread. Note that winit may still
        /// attempt to initialize COM API regardless of this option. Currently only fullscreen mode
        /// does that, but there may be more in the future. If you need COM API with
        /// `COINIT_MULTITHREADED` you must initialize it before calling any winit functions. See <https://docs.microsoft.com/en-us/windows/win32/api/objbase/nf-objbase-coinitialize#remarks> for more information.
        fn with_drag_and_drop(self, flag: bool) -> Self;

        /// Whether show or hide the window icon in the taskbar.
        fn with_skip_taskbar(self, skip: bool) -> Self;

        /// Customize the window class name.
        fn with_class_name<S: Into<String>>(self, class_name: S) -> Self;

        /// Shows or hides the background drop shadow for undecorated windows.
        ///
        /// The shadow is hidden by default.
        /// Enabling the shadow causes a thin 1px line to appear on the top of the window.
        fn with_undecorated_shadow(self, shadow: bool) -> Self;

        /// Sets system-drawn backdrop type.
        ///
        /// Requires Windows 11 build 22523+.
        fn with_system_backdrop(self, backdrop_type: BackdropType) -> Self;

        /// This sets or removes `WS_CLIPCHILDREN` style.
        fn with_clip_children(self, flag: bool) -> Self;

        /// Sets the color of the window border.
        ///
        /// Supported starting with Windows 11 Build 22000.
        fn with_border_color(self, color: Option<Color>) -> Self;

        /// Sets the background color of the title bar.
        ///
        /// Supported starting with Windows 11 Build 22000.
        fn with_title_background_color(self, color: Option<Color>) -> Self;

        /// Sets the color of the window title.
        ///
        /// Supported starting with Windows 11 Build 22000.
        fn with_title_text_color(self, color: Color) -> Self;

        /// Sets the preferred style of the window corners.
        ///
        /// Supported starting with Windows 11 Build 22000.
        fn with_corner_preference(self, corners: CornerPreference) -> Self;
    }

    impl<S> WindowOptionsExtWindows for super::WindowOptions<S> {
        #[inline]
        fn with_owner_window(mut self, parent: HWND) -> Self {
            self.initial.platform_specific.owner = Some(parent);
            self
        }

        #[inline]
        fn with_menu(mut self, menu: HMENU) -> Self {
            self.initial.platform_specific.menu = Some(menu);
            self
        }

        #[inline]
        fn with_taskbar_icon(mut self, taskbar_icon: Option<Icon>) -> Self {
            self.initial.platform_specific.taskbar_icon = taskbar_icon;
            self
        }

        #[inline]
        fn with_no_redirection_bitmap(mut self, flag: bool) -> Self {
            self.initial.platform_specific.no_redirection_bitmap = flag;
            self
        }

        #[inline]
        fn with_drag_and_drop(mut self, flag: bool) -> Self {
            self.initial.platform_specific.drag_and_drop = flag;
            self
        }

        #[inline]
        fn with_skip_taskbar(mut self, skip: bool) -> Self {
            self.reactive.platform_specific.skip_taskbar = skip;
            self
        }

        #[inline]
        fn with_class_name<C: Into<String>>(mut self, class_name: C) -> Self {
            self.initial.platform_specific.class_name = class_name.into();
            self
        }

        #[inline]
        fn with_undecorated_shadow(mut self, shadow: bool) -> Self {
            self.reactive.platform_specific.decoration_shadow = shadow;
            self
        }

        #[inline]
        fn with_system_backdrop(mut self, backdrop_type: BackdropType) -> Self {
            self.reactive.platform_specific.backdrop_type = backdrop_type;
            self
        }

        #[inline]
        fn with_clip_children(mut self, flag: bool) -> Self {
            self.initial.platform_specific.clip_children = flag;
            self
        }

        #[inline]
        fn with_border_color(mut self, color: Option<Color>) -> Self {
            self.reactive.platform_specific.border_color = Some(color.unwrap_or(NONE_COLOR));
            self
        }

        #[inline]
        fn with_title_background_color(mut self, color: Option<Color>) -> Self {
            self.reactive.platform_specific.title_background_color =
                Some(color.unwrap_or(NONE_COLOR));
            self
        }

        #[inline]
        fn with_title_text_color(mut self, color: Color) -> Self {
            self.reactive.platform_specific.title_text_color = Some(color);
            self
        }

        #[inline]
        fn with_corner_preference(mut self, corners: CornerPreference) -> Self {
            self.reactive.platform_specific.corner_preference = Some(corners);
            self
        }
    }

    const NONE_COLOR: Color = unsafe { std::mem::transmute(0xfffffffe_u32) };
}

#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
mod dummy_platform {
    use winit::window::{Window, WindowAttributes};

    #[derive(Debug, Clone, Default)]
    pub(crate) struct PlatformSpecificInitialWindowAttrs {}

    #[derive(Debug, Clone, Default)]
    pub(crate) struct PlatformSpecificReactiveWindowAttrs {}

    impl PlatformSpecificInitialWindowAttrs {
        pub(crate) fn build(&self, attrs: WindowAttributes) -> WindowAttributes {
            attrs
        }

        pub(crate) fn warn(&self, _prev: &Self) {}
    }

    impl PlatformSpecificReactiveWindowAttrs {
        pub(crate) fn build(&self, attrs: WindowAttributes) -> WindowAttributes {
            attrs
        }

        pub(crate) fn rebuild(&self, _prev: &Self, _window: &Window) {}
    }
}

#[cfg(not(windows))]
pub(crate) use dummy_platform::*;
