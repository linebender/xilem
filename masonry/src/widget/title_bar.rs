// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A titlebar widget.

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::paint_scene_helpers::fill_color;
use crate::widget::WidgetPod;
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

use super::{CenterBox, Flex, Label, WindowButton, WindowButtonType, WindowHandle};

/// A titlebar widget.
pub struct TitleBar {
    child: WidgetPod<WindowHandle>,
}

// --- MARK: BUILDERS ---
impl TitleBar {
    pub fn new() -> Self {
        let title = CenterBox::new(
            // TODO: Get the title from the window
            Label::new("Title"),
            Flex::row()
                .with_child(WindowButton::new(WindowButtonType::Minimize))
                .with_child(WindowButton::new(WindowButtonType::Maximize))
                .with_child(WindowButton::new(WindowButtonType::Close)),
        );

        let handle = WindowHandle::new(title);

        Self {
            child: WidgetPod::new(handle),
        }
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for TitleBar {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let child_size = ctx.run_layout(&mut self.child, bc);
        let size = bc.constrain((
            child_size.width,
            child_size.height.max(theme::TITLE_BAR_HEIGHT),
        ));

        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let bounds = ctx.size().to_rect();
        fill_color(scene, &bounds, theme::TITLE_BAR_COLOR);
    }

    fn accessibility_role(&self) -> Role {
        Role::TitleBar
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.child.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("TitleBar")
    }
}
