// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{FromDynWidget, NewWidget, PropertySet, Widget, WidgetMut};

use crate::ViewCtx;
use crate::core::{Mut, SuperElement, ViewElement};

/// A container for a [`Widget`] yet to be inserted in the widget tree.
///
/// This exists because the nearest equivalent type in Masonry, [`NewWidget`], can't have
/// [Xilem Core](xilem_core) traits implemented on it due to Rust's orphan rules.
///
/// If changing transforms of widgets, make sure to use [`transformed`]
/// (or [`WidgetView::transform`]).
/// This has a protocol to ensure that multiple views changing the
/// transform interoperate successfully.
///
/// [`transformed`]: crate::view::Transformed
/// [`WidgetView::transform`]: crate::view::transformed
pub struct Pod<W: Widget + FromDynWidget + ?Sized> {
    /// A [`Widget`] yet to be inserted in the widget tree.
    pub new_widget: NewWidget<W>,
}

impl<W: Widget + FromDynWidget> Pod<W> {
    /// Create a new `Pod` from a `widget`.
    ///
    /// This contains the widget value, and other metadata which will
    /// be used when that widget is added to a Masonry tree.
    pub fn new(widget: W) -> Self {
        Self {
            new_widget: NewWidget::new(widget),
        }
    }

    /// Creates a new [`Pod`] with the given `widget` and `props`.
    pub fn new_with_props(widget: W, props: impl Into<PropertySet>) -> Self {
        Self {
            new_widget: NewWidget::new_with_props(widget, props),
        }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> Pod<W> {
    /// Type-erase the contained widget.
    ///
    /// Convert a `Pod` pointing to a widget of a specific concrete type
    /// `Pod` pointing to a `dyn Widget`.
    pub fn erased(self) -> Pod<dyn Widget> {
        Pod {
            new_widget: self.new_widget.erased(),
        }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> ViewElement for Pod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for Pod<dyn Widget> {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        child.erased()
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}
