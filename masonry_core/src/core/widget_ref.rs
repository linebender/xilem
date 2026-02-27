// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use smallvec::SmallVec;
use vello::kurbo::Point;

use crate::core::{FromDynWidget, PropertiesRef, Property, QueryCtx, Widget, WidgetId};

/// A rich reference to a [`Widget`].
///
/// Widgets in Masonry are bundled with additional metadata.
///
/// A `WidgetRef` to a widget carries both a reference to the widget and to its metadata. It can [`Deref`] to the referenced widget.
///
/// This type is mostly used for debugging, to query a certain widget in the widget
/// graph, get their layout, etc. It also implements [`std::fmt::Debug`] for convenience;
/// printing it will display its widget subtree (as in, the referenced widget, and its
/// children, and their children, etc).
///
/// This is only for shared access to widgets. For widget mutation, see [`WidgetMut`](crate::core::WidgetMut).
pub struct WidgetRef<'w, W: Widget + ?Sized> {
    pub(crate) ctx: QueryCtx<'w>,
    pub(crate) widget: &'w W,
}

// --- MARK: TRAIT IMPLS

impl<W: Widget + ?Sized> Clone for WidgetRef<'_, W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<W: Widget + ?Sized> Copy for WidgetRef<'_, W> {}

impl<W: Widget + ?Sized> std::fmt::Debug for WidgetRef<'_, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let widget_name = self.widget.short_type_name();
        let display_name = if let Some(debug_text) = self.widget.get_debug_text() {
            format!("{widget_name}<{debug_text}>").into()
        } else {
            std::borrow::Cow::Borrowed(widget_name)
        };

        let children = self.children();

        if children.is_empty() {
            f.write_str(&display_name)
        } else {
            let mut f_tuple = f.debug_tuple(&display_name);
            for child in children {
                f_tuple.field(&child);
            }
            f_tuple.finish()
        }
    }
}

impl<W: Widget + ?Sized> Deref for WidgetRef<'_, W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

// --- MARK: IMPLS

impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    /// Gets a [`QueryCtx`] with information about the current widget.
    pub fn ctx(&self) -> &'_ QueryCtx<'w> {
        &self.ctx
    }

    /// Gets the actual referenced `Widget`.
    pub fn inner(self) -> &'w W {
        self.widget
    }

    /// Gets the [`WidgetId`] of the current widget.
    pub fn id(&self) -> WidgetId {
        self.ctx.widget_state.id
    }

    /// Returns `true` if the widget has a local property of type `T`.
    ///
    /// Does not check default properties.
    pub fn contains_prop<T: Property>(&self) -> bool {
        self.ctx.properties.contains::<T>()
    }

    /// Gets value of property `P`.
    ///
    /// If the widget has an entry for `P`, returns that entry.
    /// If the default property set has an entry for `P`, returns that entry.
    /// Otherwise returns [`Property::static_default()`].
    pub fn get_prop<T: Property>(&self) -> &T {
        self.ctx.properties.get::<T>()
    }

    /// Attempts to downcast to `WidgetRef` of concrete widget type.
    pub fn downcast<W2: Widget + FromDynWidget + ?Sized>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            ctx: self.ctx,
            widget: W2::from_dyn(self.widget.as_dyn())?,
        })
    }

    /// Returns refs to widget's children.
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        let parent_id = self.ctx.widget_state.id;
        self.widget
            .children_ids()
            .iter()
            .map(|&id| {
                let Some(node_ref) = self.ctx.children.into_item(id) else {
                    panic!(
                        "Error in '{}' #{parent_id}: child #{id} has not been added to tree",
                        self.widget.short_type_name()
                    );
                };

                let children = node_ref.children;
                let widget = &*node_ref.item.widget;
                let state = &node_ref.item.state;
                let properties = &node_ref.item.properties;

                let ctx = QueryCtx {
                    global_state: self.ctx.global_state,
                    widget_state: state,
                    properties: PropertiesRef {
                        set: properties,
                        default_map: self.ctx.properties.default_map,
                    },
                    children,
                    default_properties: self.ctx.default_properties,
                };

                WidgetRef { ctx, widget }
            })
            .collect()
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    /// Returns a type-erased `WidgetRef`.
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            ctx: self.ctx,
            widget: self.widget,
        }
    }
}

impl WidgetRef<'_, dyn Widget> {
    /// Recursively finds child widget with given id.
    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<Self> {
        if self.ctx.widget_state.id == id {
            Some(*self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    /// Recursively finds the innermost widget at the given position, using
    /// [`Widget::find_widget_under_pointer`] to descend the widget tree. If `self` does not contain the
    /// given position in its aligned border-box or clip path, this returns `None`.
    ///
    /// **pos** - the position is in the window's coordinate space,
    /// e.g. `(0,0)` is the top-left corner of the window.
    pub fn find_widget_under_pointer(&self, pos: Point) -> Option<Self> {
        self.widget.find_widget_under_pointer(self.ctx, pos)
    }
}
