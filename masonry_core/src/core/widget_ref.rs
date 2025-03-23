// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use smallvec::SmallVec;
use vello::kurbo::Point;

use crate::core::{PropertiesRef, QueryCtx, Widget, WidgetId};

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
    pub(crate) properties: PropertiesRef<'w>,
    pub(crate) widget: &'w W,
}

// --- MARK: TRAIT IMPLS ---

#[allow(clippy::non_canonical_clone_impl)]
impl<W: Widget + ?Sized> Clone for WidgetRef<'_, W> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

impl<W: Widget + ?Sized> Copy for WidgetRef<'_, W> {}

impl<W: Widget + ?Sized> std::fmt::Debug for WidgetRef<'_, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

// --- MARK: IMPLS ---

impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    /// Get a [`QueryCtx`] with information about the current widget.
    pub fn ctx(&self) -> &'_ QueryCtx<'w> {
        &self.ctx
    }

    /// Get the actual referenced `Widget`.
    pub fn deref(self) -> &'w W {
        self.widget
    }

    /// Get the [`WidgetId`] of the current widget.
    pub fn id(&self) -> WidgetId {
        self.ctx.widget_state.id
    }

    /// Returns `true` if the widget has a property of type `T`.
    pub fn get_prop<T: 'static>(&self) -> Option<&T> {
        self.properties.get::<T>()
    }

    /// Get value of property `T`, or `None` if the widget has no `T` property.
    pub fn contains_prop<T: 'static>(&self) -> bool {
        self.properties.contains::<T>()
    }

    /// Attempt to downcast to `WidgetRef` of concrete Widget type.
    pub fn downcast<W2: Widget>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            ctx: self.ctx,
            properties: self.properties,
            widget: self.widget.as_any().downcast_ref()?,
        })
    }

    /// Return widget's children.
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        let parent_id = self.ctx.widget_state.id;
        self.widget
            .children_ids()
            .iter()
            .map(|&id| {
                let Some(state_ref) = self.ctx.widget_state_children.into_item(id) else {
                    panic!(
                        "Error in '{}' #{parent_id}: child #{id} has not been added to tree",
                        self.widget.short_type_name()
                    );
                };
                let Some(widget_ref) = self.ctx.widget_children.into_item(id) else {
                    panic!(
                        "Error in '{}' #{parent_id}: child #{id} has not been added to tree",
                        self.widget.short_type_name()
                    );
                };
                let Some(properties_ref) = self.ctx.properties_children.into_item(id) else {
                    panic!(
                        "Error in '{}' #{parent_id}: child #{id} has not been added to tree",
                        self.widget.short_type_name()
                    );
                };

                // Box<dyn Widget> -> &dyn Widget
                // Without this step, the type of `WidgetRef::widget` would be
                // `&Box<dyn Widget> as &dyn Widget`, which would be an additional layer
                // of indirection.
                let widget = widget_ref.item;
                let widget: &dyn Widget = &**widget;

                let ctx = QueryCtx {
                    global_state: self.ctx.global_state,
                    widget_state_children: state_ref.children,
                    widget_children: widget_ref.children,
                    widget_state: state_ref.item,
                    properties_children: properties_ref.children,
                };
                let properties = PropertiesRef {
                    map: properties_ref.item,
                };

                WidgetRef {
                    ctx,
                    properties,
                    widget,
                }
            })
            .collect()
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    /// Return a type-erased `WidgetRef`.
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            ctx: self.ctx,
            properties: self.properties,
            widget: self.widget,
        }
    }
}

impl WidgetRef<'_, dyn Widget> {
    /// Recursively find child widget with given id.
    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<Self> {
        if self.ctx.widget_state.id == id {
            Some(*self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    /// Recursively find the innermost widget at the given position, using
    /// [`Widget::find_widget_at_pos`] to descend the widget tree. If `self` does not contain the
    /// given position in its layout rect or clip path, this returns `None`.
    ///
    /// **pos** - the position in global coordinates (e.g. `(0,0)` is the top-left corner of the
    /// window).
    pub fn find_widget_at_pos(&self, pos: Point) -> Option<Self> {
        self.widget
            .find_widget_at_pos(self.ctx, self.properties, pos)
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::testing::{TestHarness, TestWidgetExt as _, widget_ids};
    use crate::widgets::{Button, Label};

    #[test]
    fn downcast_ref_in_harness() {
        let [label_id] = widget_ids();
        let label = Label::new("Hello").with_id(label_id);

        let harness = TestHarness::create(label);

        assert_matches!(harness.get_widget(label_id).downcast::<Label>(), Some(_));
        assert_matches!(harness.get_widget(label_id).downcast::<Button>(), None);
    }
}
