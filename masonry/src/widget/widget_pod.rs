// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{Affine, Widget, WidgetId};

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
// TODO - Add reference to container tutorial
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
    pub(crate) transform: Affine,
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
        Self::new_with_id_and_transform(inner, id, Affine::IDENTITY)
    }

    /// Create a new widget pod with a custom transform.
    pub fn new_with_transform(inner: W, transform: Affine) -> Self {
        Self::new_with_id_and_transform(inner, WidgetId::next(), transform)
    }

    pub fn new_with_id_and_transform(inner: W, id: WidgetId, transform: Affine) -> Self {
        Self {
            id,
            inner: WidgetPodInner::Create(CreateWidget {
                widget: Box::new(inner),
                transform,
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
}

impl<W: Widget + ?Sized> WidgetPod<W> {
    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}

impl<W: Widget + 'static> WidgetPod<W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> WidgetPod<Box<dyn Widget>> {
        let WidgetPodInner::Create(inner) = self.inner else {
            panic!("Cannot box a widget after it has been inserted into the widget graph")
        };
        // TODO
        let widget: Box<dyn Widget> = inner.widget;
        WidgetPod::new_with_id_and_transform(Box::new(widget), self.id, inner.transform)
    }
}
