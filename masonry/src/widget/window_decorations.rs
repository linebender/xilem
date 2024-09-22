// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A container to show window decorations.

use accesskit::{NodeBuilder, Role};
use cursor_icon::CursorIcon;
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Insets;
use vello::Scene;
use winit::window::ResizeDirection;

use crate::paint_scene_helpers::stroke;
use crate::widget::WidgetPod;
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

const BORDER_WIDTH: f64 = 2.0;
const INSETS: Insets = Insets::uniform(BORDER_WIDTH);

/// A container to show window decorations.
pub struct WindowDecorations<W> {
    pub(crate) child: WidgetPod<W>,
}

impl<W: Widget> WindowDecorations<W> {
    pub fn new(widget: W) -> WindowDecorations<W> {
        WindowDecorations {
            child: WidgetPod::new(widget),
        }
    }
}

impl<W: Widget> Widget for WindowDecorations<W> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerMove(state) => {
                if state.position.y <= BORDER_WIDTH {
                    ctx.set_cursor(&CursorIcon::NResize);
                } else if state.position.y >= ctx.size().height - BORDER_WIDTH {
                    ctx.set_cursor(&CursorIcon::SResize);
                } else if state.position.x <= BORDER_WIDTH {
                    ctx.set_cursor(&CursorIcon::WResize);
                } else if state.position.x >= ctx.size().width - BORDER_WIDTH {
                    ctx.set_cursor(&CursorIcon::EResize);
                }
            }
            PointerEvent::PointerLeave(_) => {
                ctx.set_cursor(&CursorIcon::Default);
            }
            PointerEvent::PointerDown(_, state) => {
                if state.position.y <= BORDER_WIDTH {
                    ctx.drag_resize_window(ResizeDirection::North);
                } else if state.position.y >= ctx.size().height - BORDER_WIDTH {
                    ctx.drag_resize_window(ResizeDirection::South);
                } else if state.position.x <= BORDER_WIDTH {
                    ctx.drag_resize_window(ResizeDirection::West);
                } else if state.position.x >= ctx.size().width - BORDER_WIDTH {
                    ctx.drag_resize_window(ResizeDirection::East);
                }
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _: &mut LifeCycleCtx, _: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let padding = Size::new(INSETS.x_value(), INSETS.y_value());
        let child_bc = bc.shrink(padding);

        let child_size = ctx.run_layout(&mut self.child, &child_bc);

        let size = bc.constrain(Size::new(
            child_size.width + padding.width,
            child_size.height + padding.height,
        ));

        let child_offset = (size.to_vec2() - child_size.to_vec2()) / 2.0;
        ctx.place_child(&mut self.child, child_offset.to_point());

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let rect = ctx.size().to_rect().inset(-BORDER_WIDTH / 2.0);
        stroke(scene, &rect, theme::BORDER_DARK, BORDER_WIDTH);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.child.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("WindowDecorations")
    }
}
