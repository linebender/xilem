// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{Widget, WidgetId};

// TODO - rewrite links in doc

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// `WidgetPod` will translate internal Masonry events to regular events,
/// synthesize additional events of interest, and stop propagation when it makes sense.
pub struct WidgetPod<W> {
    id: WidgetId,
    inner: WidgetPodInner<W>,
}

// TODO - This is a simple state machine that lets users create WidgetPods
// without immediate access to the widget arena. It's *extremely* inefficient
// and leads to ugly code. The alternative is to force users to create WidgetPods
// through context methods where they already have access to the arena.
// Implementing that requires solving non-trivial design questions.

enum WidgetPodInner<W> {
    Created(W),
    Inserted,
}

impl<W: Widget> WidgetPod<W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<W> {
        Self::new_with_id(inner, WidgetId::next())
    }

    /// Create a new widget pod with fixed id.
    pub fn new_with_id(inner: W, id: WidgetId) -> WidgetPod<W> {
        WidgetPod {
            id,
            inner: WidgetPodInner::Created(inner),
        }
    }

    pub(crate) fn incomplete(&self) -> bool {
        matches!(self.inner, WidgetPodInner::Created(_))
    }

    pub(crate) fn take_inner(&mut self) -> Option<W> {
        match std::mem::replace(&mut self.inner, WidgetPodInner::Inserted) {
            WidgetPodInner::Created(widget) => Some(widget),
            WidgetPodInner::Inserted => None,
        }
    }

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
        match self.inner {
            WidgetPodInner::Created(inner) => WidgetPod::new_with_id(Box::new(inner), self.id),
            WidgetPodInner::Inserted => {
                panic!("Cannot box a widget after it has been inserted into the widget graph")
            }
        }
    }
}
