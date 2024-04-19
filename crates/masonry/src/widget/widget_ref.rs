// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::ops::Deref;

use smallvec::SmallVec;

use crate::kurbo::Point;
use crate::{Widget, WidgetId, WidgetState};

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
    widget_state: &'w WidgetState,
    widget: &'w W,
}

// --- TRAIT IMPLS ---

#[allow(clippy::non_canonical_clone_impl)]
impl<'w, W: Widget + ?Sized> Clone for WidgetRef<'w, W> {
    fn clone(&self) -> Self {
        Self {
            widget_state: self.widget_state,
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

        let children = self.widget.children();

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
    pub(crate) fn new(widget_state: &'w WidgetState, widget: &'w W) -> Self {
        WidgetRef {
            widget_state,
            widget,
        }
    }

    /// Get the [`WidgetState`] of the current widget.
    pub fn state(self) -> &'w WidgetState {
        self.widget_state
    }

    /// Get the actual referenced `Widget`.
    pub fn deref(self) -> &'w W {
        self.widget
    }

    /// Get the [`WidgetId`] of the current widget.
    pub fn id(&self) -> WidgetId {
        self.widget_state.id
    }

    /// Attempt to downcast to `WidgetRef` of concrete Widget type.
    pub fn downcast<W2: Widget>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget.as_any().downcast_ref()?,
        })
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    /// Return a type-erased `WidgetRef`.
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget,
        }
    }
}

impl<'w> WidgetRef<'w, dyn Widget> {
    /// Return widget's children.
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        self.widget.children()
    }

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

    /// Recursively find innermost widget at given position.
    ///
    /// **pos** - the position in local coordinates (zero being the top-left of the
    /// inner widget).
    pub fn find_widget_at_pos(&self, pos: Point) -> Option<WidgetRef<'w, dyn Widget>> {
        let mut pos = pos;
        let mut innermost_widget: WidgetRef<'w, dyn Widget> = *self;

        if !self.state().layout_rect().contains(pos) {
            return None;
        }

        loop {
            if let Some(child) = innermost_widget.deref().get_child_at_pos(pos) {
                pos -= innermost_widget.state().layout_rect().origin().to_vec2();
                innermost_widget = child;
            } else {
                return Some(innermost_widget);
            }
        }
    }

    /// Recursively check that the Widget tree upholds various invariants.
    ///
    /// Can only be called after on_event and lifecycle.
    pub fn debug_validate(&self, after_layout: bool) {
        if cfg!(not(debug_assertions)) {
            return;
        }

        if self.state().is_new {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget did not receive WidgetAdded",
                self.deref().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if self.state().request_focus.is_some()
            || self.state().children_changed
            || self.state().cursor.is_some()
        {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget state not cleared",
                self.deref().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if after_layout && (self.state().needs_layout || self.state().needs_window_origin) {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget layout state not cleared",
                self.deref().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        for child in self.widget.children() {
            child.debug_validate(after_layout);

            if !self.state().children.may_contain(&child.state().id) {
                debug_panic!(
                    "Widget '{}' #{} is invalid: child widget '{}' #{} not registered in children filter",
                    self.deref().short_type_name(),
                    self.state().id.to_raw(),
                    child.deref().short_type_name(),
                    child.state().id.to_raw(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt as _};
    use crate::widget::{Button, Label};
    use crate::{Widget, WidgetPod};

    #[test]
    fn downcast_ref() {
        let label = WidgetPod::new(Label::new("Hello"));
        let dyn_widget: WidgetRef<dyn Widget> = label.as_dyn();

        let label = dyn_widget.downcast::<Label>();
        assert_matches!(label, Some(_));
        let label = dyn_widget.downcast::<Button>();
        assert_matches!(label, None);
    }

    #[test]
    fn downcast_ref_in_harness() {
        let [label_id] = widget_ids();
        let label = Label::new("Hello").with_id(label_id);

        let harness = TestHarness::create(label);

        assert_matches!(harness.get_widget(label_id).downcast::<Label>(), Some(_));
        assert_matches!(harness.get_widget(label_id).downcast::<Button>(), None);
    }
}
