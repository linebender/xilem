// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::Widget;

/// A typed key which can be passed to a widget at construction, and then used to access that widget.
///
/// Unlike [`WidgetId`](crate::core::WidgetId), using this type to access a widget lets you
/// skip downcasting.
///
/// You can only add one widget with a given tag to the entire widget tree.
/// Trying to add another widget with the same tag will debug-panic or fail silently.
/// Tags currently aren't garbage-collected even when the widget is removed from the tree.
pub struct WidgetTag<W: Widget + ?Sized> {
    pub(crate) inner: WidgetTagInner,
    pub(crate) _marker: PhantomData<W>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct WidgetTagInner {
    pub(crate) id: u64,
    pub(crate) name: &'static str,
}

impl<W: Widget> WidgetTag<W> {
    /// Creates a new tag with the given name.
    ///
    /// Calling this method twice with the same string will return the same tag.
    /// Users should avoid name collisions when adding widgets with named tags.
    ///
    /// This method can be called in const contexts (e.g. to initialize a static).
    pub const fn named(name: &'static str) -> Self {
        Self {
            inner: WidgetTagInner { id: 0, name },
            _marker: PhantomData,
        }
    }

    /// Creates a new unique tag.
    ///
    /// Calling this method twice will return two different tags.
    pub fn unique() -> Self {
        static TAG_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            inner: WidgetTagInner { id, name: "" },
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
        f.debug_tuple("WidgetTag").field(&self.inner).finish()
    }
}

impl<W: Widget + ?Sized> Display for WidgetTag<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Display for WidgetTagInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.name.is_empty() {
            write!(f, "#{}", self.name)
        } else {
            write!(f, "#{}", self.id)
        }
    }
}
