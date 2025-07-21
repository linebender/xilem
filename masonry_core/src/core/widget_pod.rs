// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::Affine;

use crate::core::{Properties, Widget, WidgetId};

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
pub struct WidgetPod<W: ?Sized> {
    id: WidgetId,
    inner: WidgetPodInner<W>,
}

/// A container for a widget yet to be inserted.
#[non_exhaustive]
#[expect(missing_docs, reason = "Names are self-explanatory.")]
pub struct NewWidget<W: ?Sized> {
    pub widget: Box<W>,
    pub id: WidgetId,
    pub options: WidgetOptions,
    pub properties: Properties,
}

/// The options a new widget will be created with.
#[derive(Default, Debug)]
pub struct WidgetOptions {
    /// The transform the widget will be created with.
    pub transform: Affine,
    /// The disabled state the widget will be created with.
    pub disabled: bool,
}

// TODO - This is a simple state machine that lets users create WidgetPods
// without immediate access to the widget arena. It's very inefficient
// and leads to ugly code. The alternative is to force users to create WidgetPods
// through context methods where they already have access to the arena.
// Implementing that requires solving non-trivial design questions.

enum WidgetPodInner<W: ?Sized> {
    Create(NewWidget<W>),
    Inserted,
}

impl<W: Widget> From<W> for NewWidget<W> {
    fn from(value: W) -> Self {
        Self::new(value)
    }
}

impl<W: Widget> NewWidget<W> {
    /// Create a new widget.
    pub fn new(inner: W) -> Self {
        Self::new_with_id(inner, WidgetId::next())
    }

    /// Create a new widget with fixed id.
    pub fn new_with_id(inner: W, id: WidgetId) -> Self {
        Self {
            widget: Box::new(inner),
            id,
            options: WidgetOptions::default(),
            properties: Properties::default(),
        }
    }

    /// Create a new widget with properties.
    pub fn new_with_props(inner: W, props: Properties) -> Self {
        Self {
            widget: Box::new(inner),
            id: WidgetId::next(),
            options: WidgetOptions::default(),
            properties: props,
        }
    }

    /// Create a new widget with custom options.
    pub fn new_with_options(inner: W, options: WidgetOptions) -> Self {
        Self {
            widget: Box::new(inner),
            id: WidgetId::next(),
            options,
            properties: Properties::default(),
        }
    }

    /// Create a new widget with custom options and custom [`Properties`].
    pub fn new_with(inner: W, id: WidgetId, options: WidgetOptions, props: Properties) -> Self {
        Self {
            widget: Box::new(inner),
            id,
            options,
            properties: props,
        }
    }
}

impl<W: Widget + ?Sized> NewWidget<W> {
    /// Type-erase the contained widget.
    ///
    /// Convert a `NewWidget` pointing to a widget of a specific concrete type
    /// `NewWidget` pointing to a `dyn Widget`.
    pub fn erased(self) -> NewWidget<dyn Widget> {
        NewWidget {
            widget: self.widget.as_box_dyn(),
            id: self.id,
            options: self.options,
            properties: self.properties,
        }
    }

    /// Create a `WidgetPod` which will be added to the widget tree.
    pub fn to_pod(self) -> WidgetPod<W> {
        WidgetPod {
            id: self.id,
            inner: WidgetPodInner::Create(self),
        }
    }
}

impl<W: Widget> WidgetPod<W> {
    // FIXME - Remove
    /// Create a new widget pod.
    pub fn new(inner: W) -> Self {
        NewWidget::new(inner).to_pod()
    }
}

impl<W: Widget + ?Sized> WidgetPod<W> {
    pub(crate) fn incomplete(&self) -> bool {
        matches!(self.inner, WidgetPodInner::Create(_))
    }

    pub(crate) fn take_inner(&mut self) -> Option<NewWidget<W>> {
        match std::mem::replace(&mut self.inner, WidgetPodInner::Inserted) {
            WidgetPodInner::Create(widget) => Some(widget),
            WidgetPodInner::Inserted => None,
        }
    }

    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}
