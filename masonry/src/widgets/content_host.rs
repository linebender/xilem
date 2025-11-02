// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A dynamic single-child host that forwards to its content.

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};

/// A pass-through container that hosts exactly one child, which may be replaced dynamically.
///
/// Useful when you need an insertion point for one widget that can be
/// replaced at runtime, without adding layout or chrome of its own.
///
/// # Examples
/// Create a host and later replace its content inside a mutate/edit callback:
/// ```
/// use masonry::core::{NewWidget, Widget};
/// use masonry::widgets::{ContentHost, Label};
///
/// // Create a host around a label
/// let host = NewWidget::new(ContentHost::new(Label::new("Hello").with_auto_id()));
///
/// // ... in an edit callback, mutate the widget tree
/// # fn edit(mut host: masonry::core::WidgetMut<'_, ContentHost>) {
/// ContentHost::replace_child(&mut host, Label::new("World").with_auto_id());
/// # }
/// ```
pub struct ContentHost {
    inner: WidgetPod<dyn Widget>,
}

impl ContentHost {
    /// Create a new `ContentHost` with the given initial child.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            inner: child.erased().to_pod(),
        }
    }

    /// Replace the hosted child with a new widget.
    ///
    /// Removes the old child from the tree and installs the new child in its place.
    pub fn replace_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let old = std::mem::replace(&mut this.widget.inner, child.erased().to_pod());
        this.ctx.remove_child(old);
    }

    /// Get a mutable reference to the hosted child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.inner)
    }

    /// Get the [`WidgetId`] of the hosted child.
    pub fn inner_id(&self) -> WidgetId {
        self.inner.id()
    }
}

impl Widget for ContentHost {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.inner);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.inner, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.inner.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("ContentHost", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::{Button, Label};
    use vello::kurbo::Size;

    #[test]
    fn content_host_replaces_child() {
        // Start with a label
        let widget = NewWidget::new(ContentHost::new(Label::new("A").with_auto_id()));
        let window_size = Size::new(160.0, 60.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "content_host_initial_label_A");

        // Replace with a label with different text
        harness.edit_root_widget(|mut host| {
            ContentHost::replace_child(&mut host, Label::new("B").with_auto_id());
        });

        assert_render_snapshot!(harness, "content_host_replaced_label_B");
    }

    #[test]
    fn content_host_child_mut() {
        // Start with a label and then change text through child_mut
        let widget = NewWidget::new(ContentHost::new(Label::new("Hello").with_auto_id()));
        let window_size = Size::new(200.0, 60.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "content_host_child_mut_initial");

        harness.edit_root_widget(|mut host| {
            let mut child = ContentHost::child_mut(&mut host);
            let mut label = child.downcast::<Label>();
            Label::set_text(&mut label, "World");
        });

        assert_render_snapshot!(harness, "content_host_child_mut_updated");

        // Replace with a button to ensure dynamic type change works visually too
        harness.edit_root_widget(|mut host| {
            ContentHost::replace_child(&mut host, Button::with_text("Click").with_auto_id());
        });

        assert_render_snapshot!(harness, "content_host_child_replaced_with_button");
    }
}
