// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
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
///
/// In general, functions which take a new widget and add it to the tree
/// (e.g. `FooBarContainer::add_child_widget()`) should take a `NewWidget` as
/// a parameter.
///
/// `NewWidget` holds both the widget itself and additional metadata which will be stored
/// alongside it once it's added to the tree.
#[non_exhaustive]
pub struct NewWidget<W: ?Sized> {
    /// The widget we're going to add.
    pub widget: Box<W>,
    pub(crate) id: WidgetId,
    pub(crate) action_type: TypeId,

    /// The options the widget will be created with.
    pub options: WidgetOptions,
    /// The properties the widget will be created with.
    pub properties: Properties,
}

// TODO - Remove this and merge it into NewWidget?
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

impl<W: Widget> NewWidget<W> {
    /// Create a new widget.
    ///
    /// You can also get the same result with [`Widget::with_auto_id()`].
    pub fn new(inner: W) -> Self {
        Self::new_with_id(inner, WidgetId::next())
    }

    /// Create a new widget with pre-determined id.
    pub fn new_with_id(inner: W, id: WidgetId) -> Self {
        Self {
            widget: Box::new(inner),
            id,
            action_type: TypeId::of::<W>(),
            options: WidgetOptions::default(),
            properties: Properties::default(),
        }
    }

    // TODO - Replace with builder methods?
    /// Create a new widget with custom [`Properties`].
    pub fn new_with_props(inner: W, props: Properties) -> Self {
        Self {
            properties: props,
            ..Self::new(inner)
        }
    }

    /// Create a new widget with custom [`WidgetOptions`].
    pub fn new_with_options(inner: W, options: WidgetOptions) -> Self {
        Self {
            options,
            ..Self::new(inner)
        }
    }

    /// Create a new widget with custom [`WidgetOptions`] and custom [`Properties`].
    pub fn new_with(inner: W, id: WidgetId, options: WidgetOptions, props: Properties) -> Self {
        Self {
            widget: Box::new(inner),
            id,
            action_type: TypeId::of::<W>(),
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
            action_type: self.action_type,
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

    /// Get the id of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
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

    /// Get the id of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}
