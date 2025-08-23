// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::cursor::Cursor;
use winit::dpi::{Position, Size};
use winit::icon::Icon;
use winit::platform::wayland::{Anchor, KeyboardInteractivity, Layer, WindowAttributesWayland};
use winit::window::{Window, WindowAttributes};

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

    wayland: WindowAttributesWayland,
}

/// These are attributes the user can change, so we cannot make them reactive.
#[derive(Clone, Debug)]
pub(crate) struct InitialAttrs {
    inner_size: Option<Size>,
    position: Option<Position>,
    // TODO: move window_icon to ReactiveWindowAttrs once the winit type implements PartialEq
    window_icon: Option<Icon>,
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
                wayland: WindowAttributesWayland::default(),
            },
            initial: InitialAttrs {
                inner_size: None,
                position: None,
                window_icon: None,
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

    /// Enable Wayland layer shell support for this window.
    pub fn with_layer_shell(mut self) -> Self {
        self.reactive.wayland = self.reactive.wayland.with_layer_shell();
        self
    }

    /// Sets the anchor for a Wayland layer shell window.
    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.reactive.wayland = self.reactive.wayland.with_anchor(anchor);
        self
    }

    /// Sets the exclusive zone for a Wayland layer shell window.
    pub fn with_exclusive_zone(mut self, exclusive_zone: i32) -> Self {
        self.reactive.wayland = self.reactive.wayland.with_exclusive_zone(exclusive_zone);
        self
    }

    /// Sets the margin for a Wayland layer shell window.
    pub fn with_margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.reactive.wayland = self.reactive.wayland.with_margin(top, right, bottom, left);
        self
    }

    /// Sets the keyboard interactivity for a Wayland layer shell window.
    pub fn with_keyboard_interactivity(
        mut self,
        keyboard_interactivity: KeyboardInteractivity,
    ) -> Self {
        self.reactive.wayland = self
            .reactive
            .wayland
            .with_keyboard_interactivity(keyboard_interactivity);
        self
    }

    /// Sets the layer for a Wayland layer shell window.
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.reactive.wayland = self.reactive.wayland.with_layer(layer);
        self
    }

    pub(crate) fn build_initial_attrs(&self) -> WindowAttributes {
        let mut attrs = WindowAttributes::default()
            .with_title(self.reactive.title.clone())
            .with_cursor(self.reactive.cursor.clone())
            .with_resizable(self.reactive.resizable)
            .with_window_icon(self.initial.window_icon.clone())
            .with_platform_attributes(Box::new(self.reactive.wayland.clone()));

        if let Some(min_inner_size) = self.reactive.min_inner_size {
            attrs = attrs.with_min_surface_size(min_inner_size);
        }
        if let Some(max_inner_size) = self.reactive.max_inner_size {
            attrs = attrs.with_max_surface_size(max_inner_size);
        }
        if let Some(inner_size) = self.initial.inner_size {
            attrs = attrs.with_surface_size(inner_size);
        }
        attrs
    }

    pub(crate) fn rebuild(&self, prev: &Self, window: &dyn Window) {
        self.rebuild_reactive_window_attributes(prev, window);
        self.warn_for_changed_initial_attributes(prev);
    }

    fn rebuild_reactive_window_attributes(&self, prev: &Self, window: &dyn Window) {
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
            window.set_min_surface_size(current.min_inner_size);
        }
        if current.max_inner_size != prev.max_inner_size {
            window.set_max_surface_size(current.max_inner_size);
        }
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
    }
}
