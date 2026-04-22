// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The main shared context for a Masonry app.

use copypasta::ClipboardProvider;

/// Shared context for a Masonry application.
///
/// Provides app-wide services across different render roots.
pub struct AppCtx {
    clipboard: Box<dyn ClipboardProvider>,
}

impl core::fmt::Debug for AppCtx {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AppCtx")
    }
}

impl AppCtx {
    /// Creates a new shared context for Masonry.
    ///
    /// Only one should be created per application.
    pub fn new(clipboard: Box<dyn ClipboardProvider>) -> Self {
        Self { clipboard }
    }

    /// Sets the clipboard contents to the provided `text`.
    pub fn set_clipboard(&mut self, text: String) {
        self.clipboard.set_contents(text).unwrap();
    }

    /// Returns the current clipboard text content.
    pub fn get_clipboard(&mut self) -> String {
        self.clipboard.get_contents().unwrap()
    }
}
