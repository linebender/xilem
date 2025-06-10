// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{
    FromDynWidget, Properties, Widget, WidgetId, WidgetMut, WidgetOptions, WidgetPod,
};

use crate::ViewCtx;
use crate::core::{Mut, SuperElement, ViewElement};

/// A container for a yet to be inserted [Masonry](masonry) widget
/// to be used with Xilem.
///
/// This exists for two reasons:
/// 1) The nearest equivalent type in Masonry, [`WidgetPod`], can't have
///    [Xilem Core](xilem_core) traits implemented on it due to Rust's orphan rules.
/// 2) `WidgetPod` is also used during a Widget's lifetime to contain its children,
///    and so might not actually own the underlying widget value.
///    When creating widgets in Xilem, layered views all want access to the - using
///    `WidgetPod` for this purpose would require fallible unwrapping.
#[expect(missing_docs, reason = "TODO - Document these items")]
pub struct Pod<W: Widget + FromDynWidget + ?Sized> {
    pub widget: Box<W>,
    pub id: WidgetId,
    /// The options the widget will be created with.
    ///
    /// If changing transforms of widgets, prefer to use [`transformed`]
    /// (or [`WidgetView::transform`]).
    /// This has a protocol to ensure that multiple views changing the
    /// transform interoperate successfully.
    ///
    /// [`transformed`]: crate::view::Transformed
    /// [`WidgetView::transform`]: crate::view::transformed
    pub options: WidgetOptions,
    pub properties: Properties,
}

impl<W: Widget + FromDynWidget> Pod<W> {
    /// Create a new `Pod` from a `widget`.
    ///
    /// This contains the widget value, and other metadata which will
    /// be used when that widget is added to a Masonry tree.
    pub fn new(widget: W) -> Self {
        Self {
            widget: Box::new(widget),
            id: WidgetId::next(),
            options: WidgetOptions::default(),
            properties: Properties::new(),
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
            widget: self.widget.as_box_dyn(),
            id: self.id,
            options: self.options,
            properties: self.properties,
        }
    }
    /// Finalise this `Pod`, converting into a [`WidgetPod`].
    ///
    /// In most cases, you will use the return value when creating a
    /// widget with a single child.
    /// For example, button widgets have a label child.
    ///
    /// If you're adding the widget to a layout container widget,
    /// which can contain heterogenous widgets, you will probably
    /// prefer to use [`Self::erased_widget_pod`].
    pub fn into_widget_pod(self) -> WidgetPod<W> {
        WidgetPod::new_with(self.widget, self.id, self.options, self.properties)
    }
    /// Finalise this `Pod` into a type-erased [`WidgetPod`].
    ///
    /// In most cases, you will use the return value for adding to a layout
    /// widget which supports heterogenous widgets.
    /// For example, [`Flex`](masonry::widgets::Flex) accepts type-erased widget pods.
    pub fn erased_widget_pod(self) -> WidgetPod<dyn Widget> {
        WidgetPod::new_with(self.widget, self.id, self.options, self.properties).erased()
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
