use crate::{
    vello::Scene, widget::WidgetMut, AccessCtx, BoxConstraints, LayoutCtx, PaintCtx, Point,
    RegisterCtx, Size, Widget, WidgetId, WidgetPod,
};
use accesskit::{NodeBuilder, Role};
use smallvec::SmallVec;
use tracing::trace_span;

struct Child {
    widget: WidgetPod<Box<dyn Widget>>,
}

#[derive(Default)]
pub struct ZStack {
    children: Vec<Child>,
}

// --- MARK: IMPL ZSTACK ---
impl ZStack {
    pub fn new() -> Self {
        ZStack {
            children: Vec::new(),
        }
    }

    pub fn with_child(self, child: impl Widget) -> Self {
        self.with_child_pod(WidgetPod::new(Box::new(child)))
    }

    pub fn with_child_id(self, child: impl Widget, id: WidgetId) -> Self {
        self.with_child_pod(WidgetPod::new_with_id(Box::new(child), id))
    }

    pub fn with_child_pod(mut self, child: WidgetPod<Box<dyn Widget>>) -> Self {
        let child = Child { widget: child };
        self.children.push(child);
        self
    }
}

// --- MARK: WIDGETMUT---
impl ZStack {
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: impl Widget) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new(Box::new(child));
        Self::insert_child_pod(this, child_pod);
    }

    pub fn add_child_id(this: &mut WidgetMut<'_, Self>, child: impl Widget, id: WidgetId) {
        let child_pod: WidgetPod<Box<dyn Widget>> = WidgetPod::new_with_id(Box::new(child), id);
        Self::insert_child_pod(this, child_pod);
    }

    pub fn insert_child_pod(this: &mut WidgetMut<'_, Self>, widget: WidgetPod<Box<dyn Widget>>) {
        let child = Child { widget };
        this.widget.children.push(child);
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
        this.ctx.request_layout();
    }

    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> Option<WidgetMut<'t, Box<dyn Widget>>> {
        let child = &mut this.widget.children[idx].widget;
        Some(this.ctx.get_mut(child))
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for ZStack {
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let total_size = bc.max();

        for child in &mut self.children {
            let _child_size = ctx.run_layout(&mut child.widget, bc);
            ctx.place_child(&mut child.widget, Point::ZERO);
        }

        total_size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        for child in self.children.iter_mut().map(|x| &mut x.widget) {
            ctx.register_child(child);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.children
            .iter()
            .map(|child| &child.widget)
            .map(|widget_pod| widget_pod.id())
            .collect()
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn make_trace_span(&self) -> tracing::Span {
        trace_span!("ZStack")
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::{Button, Label};

    #[test]
    fn zstack_with_button_and_label() {
        let widget = ZStack::new()
            .with_child(Button::new("Button"))
            .with_child(Label::new("Label"));

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "zstack_with_button_and_label");
    }
}
