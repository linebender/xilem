// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Properties, Widget, WidgetId, WidgetOptions};

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
pub struct WidgetPod<W: ?Sized> {
    id: WidgetId,
    inner: WidgetPodInner<W>,
}

// TODO - This is a simple state machine that lets users create WidgetPods
// without immediate access to the widget arena. It's very inefficient
// and leads to ugly code. The alternative is to force users to create WidgetPods
// through context methods where they already have access to the arena.
// Implementing that requires solving non-trivial design questions.

pub(crate) struct CreateWidget<W: ?Sized> {
    pub(crate) widget: Box<W>,
    pub(crate) options: WidgetOptions,
    pub(crate) properties: Properties,
}

enum WidgetPodInner<W: ?Sized> {
    Create(CreateWidget<W>),
    Inserted,
}

impl<W: Widget> WidgetPod<W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> Self {
        Self::new_with_id(inner, WidgetId::next())
    }

    /// Create a new widget pod with fixed id.
    pub fn new_with_id(inner: W, id: WidgetId) -> Self {
        Self::new_with_id_and_options(Box::new(inner), id, WidgetOptions::default())
    }
}

impl<W: Widget + ?Sized> WidgetPod<W> {
    /// Create a new widget pod with custom options.
    pub fn new_with_options(inner: Box<W>, options: WidgetOptions) -> Self {
        Self::new_with_id_and_options(inner, WidgetId::next(), options)
    }

    /// Create a new widget pod with custom options and a pre-set [`WidgetId`].
    pub fn new_with_id_and_options(inner: Box<W>, id: WidgetId, options: WidgetOptions) -> Self {
        Self {
            id,
            inner: WidgetPodInner::Create(CreateWidget {
                widget: inner,
                options,
                properties: Properties::new(),
            }),
        }
    }

    /// Create a new widget pod with custom options and custom [`Properties`].
    pub fn new_with(
        inner: Box<W>,
        id: WidgetId,
        options: WidgetOptions,
        props: Properties,
    ) -> Self {
        Self {
            id,
            inner: WidgetPodInner::Create(CreateWidget {
                widget: inner,
                options,
                properties: props,
            }),
        }
    }

    pub(crate) fn incomplete(&self) -> bool {
        matches!(self.inner, WidgetPodInner::Create(_))
    }

    pub(crate) fn take_inner(&mut self) -> Option<CreateWidget<W>> {
        match std::mem::replace(&mut self.inner, WidgetPodInner::Inserted) {
            WidgetPodInner::Create(widget) => Some(widget),
            WidgetPodInner::Inserted => None,
        }
    }

    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Type-erase the contained widget.
    ///
    /// Convert a `WidgetPod` pointing to a widget of a specific concrete type
    /// `WidgetPod` pointing to a `dyn Widget`.
    pub fn erased(self) -> WidgetPod<dyn Widget> {
        let WidgetPodInner::Create(inner) = self.inner else {
            // TODO - Enabling this case isn't impossible anymore.
            // We're keeping it forbidden for now.
            panic!("Cannot box a widget after it has been inserted into the widget graph")
        };
        WidgetPod {
            id: self.id,
            inner: WidgetPodInner::Create(CreateWidget {
                widget: inner.widget.as_box_dyn(),
                options: inner.options,
                properties: inner.properties,
            }),
        }
    }
}
