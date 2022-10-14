use std::ops::Deref;

use smallvec::SmallVec;

use crate::kurbo::Point;
use crate::{Widget, WidgetId, WidgetState};

// ---
pub struct WidgetRef<'w, W: Widget + ?Sized> {
    pub widget_state: &'w WidgetState,
    pub widget: &'w W,
}

// TODO - Make sure WidgetRef and WidgetMut have the same utility methods.
// TODO - Document
impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    pub fn new(widget_state: &'w WidgetState, widget: &'w W) -> Self {
        WidgetRef {
            widget_state,
            widget,
        }
    }

    pub fn state(self) -> &'w WidgetState {
        self.widget_state
    }

    pub fn widget(self) -> &'w W {
        self.widget
    }

    /// get the `WidgetId` of the current widget.
    pub fn id(&self) -> WidgetId {
        self.widget_state.id
    }
}

// --- TRAIT IMPLS ---

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
        &self.widget
    }
}

impl<'w, W: Widget> WidgetRef<'w, W> {
    // TODO - document
    pub fn as_dyn(&self) -> WidgetRef<'w, dyn Widget> {
        WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget,
        }
    }
}

impl<'w, W: Widget + ?Sized> WidgetRef<'w, W> {
    // TODO - document
    pub fn downcast<W2: Widget>(&self) -> Option<WidgetRef<'w, W2>> {
        Some(WidgetRef {
            widget_state: self.widget_state,
            widget: self.widget.as_any().downcast_ref()?,
        })
    }
}

impl<'w> WidgetRef<'w, dyn Widget> {
    pub fn children(&self) -> SmallVec<[WidgetRef<'w, dyn Widget>; 16]> {
        self.widget.children()
    }

    pub fn find_widget_by_id(&self, id: WidgetId) -> Option<WidgetRef<'w, dyn Widget>> {
        if self.state().id == id {
            Some(*self)
        } else {
            self.children()
                .into_iter()
                .find_map(|child| child.find_widget_by_id(id))
        }
    }

    pub fn find_widget_at_pos(&self, pos: Point) -> Option<WidgetRef<'w, dyn Widget>> {
        let mut pos = pos;
        let mut innermost_widget: WidgetRef<'w, dyn Widget> = *self;

        if !self.state().layout_rect().contains(pos) {
            return None;
        }

        // FIXME - Handle hidden widgets (eg in scroll areas).
        loop {
            if let Some(child) = innermost_widget.widget().get_child_at_pos(pos) {
                pos -= innermost_widget.state().layout_rect().origin().to_vec2();
                innermost_widget = child;
            } else {
                return Some(innermost_widget);
            }
        }
    }

    // TODO - reorganize this part of the code
    pub(crate) fn prepare_pass(&self) {
        self.state().mark_as_visited(false);
    }

    // can only be called after on_event and lifecycle
    // checks that basic invariants are held
    pub fn debug_validate(&self, after_layout: bool) {
        if cfg!(not(debug_assertions)) {
            return;
        }

        if self.state().is_new {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget did not receive WidgetAdded",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if self.state().request_focus.is_some()
            || self.state().children_changed
            || self.state().cursor.is_some()
        {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget state not cleared",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        if after_layout && (self.state().needs_layout || self.state().needs_window_origin) {
            debug_panic!(
                "Widget '{}' #{} is invalid: widget layout state not cleared",
                self.widget().short_type_name(),
                self.state().id.to_raw(),
            );
        }

        for child in self.widget.children() {
            child.debug_validate(after_layout);

            if !self.state().children.may_contain(&child.state().id) {
                debug_panic!(
                    "Widget '{}' #{} is invalid: child widget '{}' #{} not registered in children filter",
                    self.widget().short_type_name(),
                    self.state().id.to_raw(),
                    child.widget().short_type_name(),
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
    use crate::testing::{widget_ids, Harness, TestWidgetExt as _};
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

        let harness = Harness::create(label);

        assert_matches!(harness.get_widget(label_id).downcast::<Label>(), Some(_));
        assert_matches!(harness.get_widget(label_id).downcast::<Button>(), None);
    }
}
