// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use crate::core::Widget;

/// A typed key which can be passed to a widget at construction, and then used to access that widget.
///
/// Unlike [`WidgetId`](crate::core::WidgetId), using this type to access a widget lets you
/// skip downcasting.
/// This should mostly be useful for testing.
///
/// You can only add one widget with a given tag to the entire widget tree.
/// Trying to add another widget with the same tag will debug-panic or fail silently.
/// Tags currently aren't garbage-collected even when the widget is removed from the tree.
pub struct WidgetTag<W: Widget + ?Sized> {
    pub(crate) name: &'static str,
    pub(crate) _marker: PhantomData<W>,
}

impl<W: Widget> WidgetTag<W> {
    /// Create a new tag.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }
}

// Some of the impls could be derived, but then the bounds would be too restrictive.

impl<W: Widget + ?Sized> Clone for WidgetTag<W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<W: Widget + ?Sized> Copy for WidgetTag<W> {}

impl<W: Widget + ?Sized> Debug for WidgetTag<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetTag")
            .field("name", &self.name)
            .finish()
    }
}

impl<W: Widget + ?Sized> Display for WidgetTag<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}
