use crate::{
    vello::Scene, widget::WidgetMut, AccessCtx, BoxConstraints, LayoutCtx, PaintCtx, Point,
    QueryCtx, RegisterCtx, Size, Widget, WidgetId, WidgetPod,
};
use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::trace_span;

struct Child {
    widget: WidgetPod<Box<dyn Widget>>,
}

#[derive(Default)]
pub struct ZStack {
    children: Vec<Child>,
    alignment: Alignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    TopLeading,
    Top,
    TopTrailing,
    Leading,
    Center,
    Trailing,
    BottomLeading,
    Bottom,
    BottomTrailing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Leading,
    Center,
    Trailing,
}

// --- MARK: IMPL ALIGNMENTS ---

impl Alignment {
    pub fn new(vertical: VerticalAlignment, horizontal: HorizontalAlignment) -> Self {
        match (vertical, horizontal) {
            (VerticalAlignment::Top, HorizontalAlignment::Leading) => Self::TopLeading,
            (VerticalAlignment::Top, HorizontalAlignment::Center) => Self::Top,
            (VerticalAlignment::Top, HorizontalAlignment::Trailing) => Self::TopTrailing,
            (VerticalAlignment::Center, HorizontalAlignment::Leading) => Self::Leading,
            (VerticalAlignment::Center, HorizontalAlignment::Center) => Self::Center,
            (VerticalAlignment::Center, HorizontalAlignment::Trailing) => Self::Trailing,
            (VerticalAlignment::Bottom, HorizontalAlignment::Leading) => Self::BottomLeading,
            (VerticalAlignment::Bottom, HorizontalAlignment::Center) => Self::Bottom,
            (VerticalAlignment::Bottom, HorizontalAlignment::Trailing) => Self::BottomTrailing,
        }
    }

    pub fn vertical(self) -> VerticalAlignment {
        match self {
            Alignment::Center | Alignment::Leading | Alignment::Trailing => {
                VerticalAlignment::Center
            }
            Alignment::Top | Alignment::TopLeading | Alignment::TopTrailing => {
                VerticalAlignment::Top
            }
            Alignment::Bottom | Alignment::BottomLeading | Alignment::BottomTrailing => {
                VerticalAlignment::Bottom
            }
        }
    }

    pub fn horizontal(self) -> HorizontalAlignment {
        match self {
            Alignment::Center | Alignment::Top | Alignment::Bottom => HorizontalAlignment::Center,
            Alignment::Leading | Alignment::TopLeading | Alignment::BottomLeading => {
                HorizontalAlignment::Leading
            }
            Alignment::Trailing | Alignment::TopTrailing | Alignment::BottomTrailing => {
                HorizontalAlignment::Trailing
            }
        }
    }
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Center
    }
}

impl Default for VerticalAlignment {
    fn default() -> Self {
        Self::Center
    }
}

impl Default for HorizontalAlignment {
    fn default() -> Self {
        Self::Center
    }
}

impl From<Alignment> for VerticalAlignment {
    fn from(value: Alignment) -> Self {
        value.vertical()
    }
}

impl From<Alignment> for HorizontalAlignment {
    fn from(value: Alignment) -> Self {
        value.horizontal()
    }
}

impl From<(VerticalAlignment, HorizontalAlignment)> for Alignment {
    fn from((vertical, horizontal): (VerticalAlignment, HorizontalAlignment)) -> Self {
        Alignment::new(vertical, horizontal)
    }
}

impl From<VerticalAlignment> for Alignment {
    fn from(vertical: VerticalAlignment) -> Self {
        Alignment::new(vertical, HorizontalAlignment::Center)
    }
}

impl From<HorizontalAlignment> for Alignment {
    fn from(horizontal: HorizontalAlignment) -> Self {
        Alignment::new(VerticalAlignment::Center, horizontal)
    }
}

// --- MARK: IMPL ZSTACK ---
impl ZStack {
    pub fn new() -> Self {
        ZStack {
            children: Vec::new(),
            alignment: Alignment::default(),
        }
    }

    pub fn with_alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
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

    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: impl Into<Alignment>) {
        this.widget.alignment = alignment.into();
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET---
impl Widget for ZStack {
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let total_size = bc.max();

        for child in &mut self.children {
            let child_bc = bc.loosen();
            let child_size = ctx.run_layout(&mut child.widget, &child_bc);

            let end = total_size - child_size;
            let end = Point::new(end.width, end.height);

            let center = Point::new(end.x / 2., end.y / 2.);

            let origin = match self.alignment {
                Alignment::TopLeading => Point::ZERO,
                Alignment::Top => Point::new(center.x, 0.),
                Alignment::TopTrailing => Point::new(end.x, 0.),
                Alignment::Leading => Point::new(0., center.y),
                Alignment::Center => center,
                Alignment::Trailing => Point::new(end.x, center.y),
                Alignment::BottomLeading => Point::new(0., end.y),
                Alignment::Bottom => Point::new(center.x, end.y),
                Alignment::BottomTrailing => end,
            };

            ctx.place_child(&mut child.widget, origin);
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

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut Node) {}

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> tracing::Span {
        trace_span!("ZStack", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use vello::peniko::Color;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::{Label, SizedBox};

    #[test]
    fn zstack_alignments() {
        let widget = ZStack::new()
            .with_child(
                SizedBox::new(Label::new("Background"))
                    .width(200.)
                    .height(100.)
                    .background(Color::BLUE)
                    .border(Color::TEAL, 2.),
            )
            .with_child(
                SizedBox::new(Label::new("Foreground"))
                    .background(Color::RED)
                    .border(Color::PINK, 2.),
            );

        let mut harness = TestHarness::create(widget);
        assert_render_snapshot!(harness, "zstack_alignment_default");

        let vertical_cases = [
            ("top", VerticalAlignment::Top),
            ("center", VerticalAlignment::Center),
            ("bottom", VerticalAlignment::Bottom),
        ];

        let horizontal_cases = [
            ("leading", HorizontalAlignment::Leading),
            ("center", HorizontalAlignment::Center),
            ("trailing", HorizontalAlignment::Trailing),
        ];

        let all_cases = vertical_cases
            .into_iter()
            .flat_map(|vert| horizontal_cases.map(|hori| (vert, hori)));

        for (vertical, horizontal) in all_cases {
            harness.edit_root_widget(|mut zstack| {
                let mut zstack = zstack.downcast::<ZStack>();
                ZStack::set_alignment(&mut zstack, (vertical.1, horizontal.1));
            });
            assert_render_snapshot!(
                harness,
                &format!("zstack_alignment_{}_{}", vertical.0, horizontal.0)
            );
        }
    }
}
