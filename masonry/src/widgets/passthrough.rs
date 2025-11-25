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
/// use masonry::widgets::{Passthrough, Label};
///
/// // Create a host around a label
/// let host = NewWidget::new(Passthrough::new(Label::new("Hello").with_auto_id()));
///
/// // ... in an edit callback, mutate the widget tree
/// # fn edit(mut host: masonry::core::WidgetMut<'_, Passthrough>) {
/// Passthrough::set_child(&mut host, Label::new("World").with_auto_id());
/// # }
/// ```
pub struct Passthrough {
    inner: WidgetPod<dyn Widget>,
}

// --- MARK: BUILDERS
impl Passthrough {
    /// Create a new `Passthrough` with the given initial child.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            inner: child.erased().to_pod(),
        }
    }
}

// --- MARK: METHODS
impl Passthrough {
    /// Get the [`WidgetId`] of the hosted child.
    pub fn inner_id(&self) -> WidgetId {
        self.inner.id()
    }
}

// --- MARK: WIDGETMUT
impl Passthrough {
    /// Replace the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let old = std::mem::replace(&mut this.widget.inner, child.erased().to_pod());
        this.ctx.remove_child(old);
    }

    /// Get a mutable reference to the hosted child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.inner)
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Passthrough {
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
        trace_span!("Passthrough", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Label;
    use vello::kurbo::Size;

    #[test]
    fn passthrough_replaces_child() {
        // Start with a label
        let widget = NewWidget::new(Passthrough::new(Label::new("A").with_auto_id()));
        let window_size = Size::new(30.0, 30.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "passthrough_initial_label_A");

        harness.edit_root_widget(|mut host| {
            let mut child = Passthrough::child_mut(&mut host);

            // Test that child_mut returns a pointer to the child label
            let _ = child.downcast::<Label>();
        });

        // Replace with a label with different text
        harness.edit_root_widget(|mut host| {
            Passthrough::set_child(&mut host, Label::new("B").with_auto_id());
        });

        assert_render_snapshot!(harness, "passthrough_replaced_label_B");
    }
}
