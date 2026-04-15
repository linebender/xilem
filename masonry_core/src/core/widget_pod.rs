// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::TypeId, collections::HashSet};

use kurbo::Affine;

use crate::core::{PropertySet, PropertyStackId, Widget, WidgetId, WidgetTag, WidgetTagInner};

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
pub struct WidgetPod<W: Widget + ?Sized> {
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
    #[cfg(debug_assertions)]
    pub(crate) action_type_name: &'static str,

    /// The options the widget will be created with.
    pub options: WidgetOptions,
    /// The properties the widget will be created with.
    pub properties: PropertySet,
    /// The id of the cascading stack of properties that will be applied to this widget, if any.
    pub property_stack_id: Option<PropertyStackId>,
    /// The classes the widget will be created with.
    pub classes: HashSet<String>,

    pub(crate) tag: Option<WidgetTagInner>,
}

impl<W: Widget + ?Sized> std::fmt::Debug for WidgetPod<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetPod")
            .field("id", &self.id)
            .field("inner", &self.inner)
            .finish()
    }
}

impl<W: Widget + ?Sized> std::fmt::Debug for WidgetPodInner<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create(arg0) => f.debug_tuple("Create").field(arg0).finish(),
            Self::Inserted => write!(f, "Inserted"),
        }
    }
}

impl<W: Widget + ?Sized> std::fmt::Debug for NewWidget<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewWidget")
            .field("widget_type", &self.widget.short_type_name())
            .field("id", &self.id)
            .field("options", &self.options)
            .field("tag", &self.tag)
            .field("classes", &self.classes)
            .finish_non_exhaustive()
    }
}

// TODO - Remove this and merge it into NewWidget?
/// The options a new widget will be created with.
#[derive(Default, Debug)]
pub struct WidgetOptions {
    /// Local transform used during the mapping of this widget's border-box coordinate space
    /// to the parent's border-box coordinate space.
    ///
    /// When calculating the effective border-box of this widget, first this transform
    /// will be applied and then `scroll_translation` and `origin` applied on top.
    pub transform: Affine,
    /// The disabled state the widget will be created with.
    pub disabled: bool,
}

// TODO - This is a simple state machine that lets users create WidgetPods
// without immediate access to the widget arena. It's very inefficient
// and leads to ugly code. The alternative is to force users to create WidgetPods
// through context methods where they already have access to the arena.
// Implementing that requires solving non-trivial design questions.

enum WidgetPodInner<W: Widget + ?Sized> {
    Create(Box<NewWidget<W>>),
    Inserted,
}

impl<W: Widget> NewWidget<W> {
    /// Creates a new widget.
    ///
    /// You can also get the same result with [`Widget::prepare()`].
    #[inline(always)]
    pub fn new(inner: W) -> Self {
        Self {
            widget: Box::new(inner),
            id: WidgetId::next(),
            action_type: TypeId::of::<W::Action>(),
            #[cfg(debug_assertions)]
            action_type_name: std::any::type_name::<W::Action>(),
            options: WidgetOptions::default(),
            properties: PropertySet::default(),
            property_stack_id: None,
            classes: HashSet::new(),
            tag: None,
        }
    }
}

impl<W: Widget + ?Sized> NewWidget<W> {
    /// Converts a `NewWidget` pointing to a widget of a specific concrete type
    /// to a `NewWidget` pointing to a `dyn Widget`.
    pub fn erased(self) -> NewWidget<dyn Widget> {
        NewWidget {
            widget: self.widget.as_box_dyn(),
            id: self.id,
            action_type: self.action_type,
            #[cfg(debug_assertions)]
            action_type_name: self.action_type_name,
            options: self.options,
            properties: self.properties,
            property_stack_id: self.property_stack_id,
            tag: self.tag,
            classes: self.classes,
        }
    }

    /// Sets the [`WidgetTag`] to this widget.
    pub fn with_tag(mut self, tag: WidgetTag<W>) -> Self {
        self.tag = Some(tag.inner);
        self
    }

    /// Sets the [`PropertySet`] for this widget.
    pub fn with_props(mut self, props: impl Into<PropertySet>) -> Self {
        self.properties = props.into();
        self
    }

    /// Sets the [`Affine`] transform for this widget.
    pub fn with_transform(mut self, transform: Affine) -> Self {
        self.options.transform = transform;
        self
    }

    /// Assigns a [`PropertyStack`](crate::core::PropertyStack) to this widget.
    pub fn with_property_stack(mut self, id: PropertyStackId) -> Self {
        self.property_stack_id = Some(id);
        self
    }

    /// Adds [class] to this widget.
    ///
    /// [class]: crate::doc::masonry_concepts#classes
    pub fn with_class(mut self, class: &str) -> Self {
        self.classes.insert(class.to_string());
        self
    }

    /// Adds [classes] to this widget.
    ///
    /// [classes]: crate::doc::masonry_concepts#classes
    pub fn with_classes(mut self, classes: impl IntoIterator<Item = String>) -> Self {
        self.classes.extend(classes);
        self
    }

    /// Set whether the widget will be created in a [disabled] state.
    ///
    /// [disabled]: crate::doc::masonry_concepts#disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.options.disabled = disabled;
        self
    }

    /// Creates a `WidgetPod` which will be added to the widget tree.
    pub fn to_pod(self) -> WidgetPod<W> {
        WidgetPod {
            id: self.id,
            inner: WidgetPodInner::Create(Box::new(self)),
        }
    }

    /// Returns the id of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}

impl<W: Widget> WidgetPod<W> {
    // FIXME - Remove
    /// Creates a new widget pod.
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
            WidgetPodInner::Create(widget) => Some(*widget),
            WidgetPodInner::Inserted => None,
        }
    }

    /// Returns the id of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }
}
