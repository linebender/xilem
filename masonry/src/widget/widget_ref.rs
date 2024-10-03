// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use smallvec::SmallVec;
use vello::kurbo::Point;

use crate::{QueryCtx, Widget, WidgetId, WidgetState};

/// A rich reference to a [`Widget`].
///
/// Widgets in Masonry are bundled with additional metadata called [`WidgetState`].
///
/// A `WidgetRef` to a widget carries both a reference to the widget and to its `WidgetState`. It can [`Deref`] to the referenced widget.
///
/// This type is mostly used for debugging, to query a certain widget in the widget
/// graph, get their layout, etc. It also implements [`std::fmt::Debug`] for convenience;
/// printing it will display its widget subtree (as in, the referenced widget, and its
/// children, and their children, etc).
///
/// This is only for shared access to widgets. For widget mutation, see [`WidgetMut`](crate::widget::WidgetMut).
pub struct WidgetRef<'w, W: Widget + ?Sized> {
    pub(crate) ctx: QueryCtx<'w>,
    pub(crate) widget: &'w W,
}

// --- TRAIT IMPLS ---

#[allow(clippy::non_canonical_clone_impl)]
impl<'w, W: Widget + ?Sized> Clone for WidgetRef<'w, W> {
    fn clone(&self) -> Self {
        Self {
            ctx: self.ctx,
            widget: self.widget,
        }
    }
}

impl<'w, W: Widget + ?Sized> Copy for WidgetRef<'w, W> {}

impl<'w, W: Widget + ?Sized> std::fmt::Debug for WidgetRef<'w, W> {
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

impl<'w, W: Widget + ?Sized> Deref for WidgetRef<'w, W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

// --- IMPLS ---

impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    // TODO - Replace with individual methods from WidgetState
    /// Get the [`WidgetState`] of the current widget.
    pub fn state(self) -> &'w WidgetState {
        self.ctx.widget_state
    }

    /// Get the actual referenced `Widget`.
    pub fn deref(self) -> &'w W {
        self.widget
    }

    /// Get the [`WidgetId`] of the current widget.
    pub fn id(&self) -> WidgetId {
        self.ctx.widget_state.id
    }

    /// Attempt to downcast to `WidgetRef` of concrete Widget type.
    pub fn downcast<W2: Widget>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            ctx: self.ctx,
            widget: self.widget.as_any().downcast_ref()?,
        })
    }

    /// Return widget's children.
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        let parent_id = self.ctx.widget_state.id.to_raw();
        self.widget
            .children_ids()
            .iter()
            .map(|id| {
                let id = id.to_raw();
                let Some(state_ref) = self.ctx.widget_state_children.into_child(id) else {
                    panic!(
                        "Error in '{}' #{parent_id}: child #{id} has not been added to tree",
                        self.widget.short_type_name()
                    );
                };
                let Some(widget_ref) = self.ctx.widget_children.into_child(id) else {
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
                };

                WidgetRef { ctx, widget }
            })
            .collect()
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    /// Return a type-erased `WidgetRef`.
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            ctx: self.ctx,
            widget: self.widget,
        }
    }
}

impl<'w> WidgetRef<'w, dyn Widget> {
    /// Recursively find child widget with given id.
    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<WidgetRef<'w, dyn Widget>> {
        if self.state().id == id {
            Some(*self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    /// Recursively find the innermost widget at the given position, using
    /// [`Widget::get_child_at_pos`] to descend the widget tree. If `self` does not contain the
    /// given position in its layout rect or clip path, this returns `None`.
    ///
    /// **pos** - the position in global coordinates (e.g. `(0,0)` is the top-left corner of the
    /// window).
    pub fn find_widget_at_pos(&self, pos: Point) -> Option<WidgetRef<'_, dyn Widget>> {
        let mut innermost_widget = *self;

        if !self.state().window_layout_rect().contains(pos) {
            return None;
        }

        // TODO: add debug assertion to check whether the child returned by
        // `Widget::get_child_at_pos` upholds the conditions of that method. See
        // https://github.com/linebender/xilem/pull/565#discussion_r1756536870
        while let Some(child) = innermost_widget
            .widget
            .get_child_at_pos(innermost_widget.ctx, pos)
        {
            innermost_widget = child;
        }

        Some(innermost_widget)
    }

    /// Recursively check that the Widget tree upholds various invariants.
    ///
    /// Can only be called after `on_event` and `lifecycle`.
    pub fn debug_validate(&self, after_layout: bool) {
        if cfg!(not(debug_assertions)) {
            return;
        }

        // TODO
        #[cfg(FALSE)]
        if self.state().is_new {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget did not receive WidgetAdded",
                self.deref().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if after_layout && self.state().needs_layout {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget layout state not cleared",
                self.deref().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        for child in self.children() {
            child.debug_validate(after_layout);
        }
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::testing::{widget_ids, TestHarness, TestWidgetExt as _};
    use crate::widget::{Button, Label};

    #[test]
    fn downcast_ref_in_harness() {
        let [label_id] = widget_ids();
        let label = Label::new("Hello").with_id(label_id);

        let harness = TestHarness::create(label);

        assert_matches!(harness.get_widget(label_id).downcast::<Label>(), Some(_));
        assert_matches!(harness.get_widget(label_id).downcast::<Button>(), None);
    }
}
