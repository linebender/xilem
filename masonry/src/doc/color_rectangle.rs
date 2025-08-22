// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// This file is the template for the ColorRectangle examples in docs.
// Because these examples each include chunklets of code, it's hard to make them compile.
// Instead, make sure this file compiles, and whenever you change this file, make sure to
// also mirror the change in the respective examples.

// TODO - Find some way to check that code chunks in docs
// are up to date with this file.

#![expect(missing_docs, reason = "This is example code")]

use crate as masonry;

// Note: The "// ---" lines separate blocks of code which are included together in
// a tutorial example. So for example, the first code block in the widget tutorial
// imports Color and Size, and then successive code blocks import more items.

use masonry::kurbo::Size;
use masonry::peniko::Color;
// ---
use masonry::core::{
    AccessEvent, EventCtx, PointerButton, PointerEvent, PropertiesMut, TextEvent, Widget,
};
// ---
use masonry::core::{Update, UpdateCtx};
// ---
use masonry::core::{BoxConstraints, LayoutCtx};
// ---
use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, PaintCtx, PropertiesRef};
use masonry::kurbo::Affine;
use masonry::peniko::Fill;
use masonry::vello::Scene;
// ---
use masonry::core::WidgetId;
use tracing::{Span, trace_span};
// ---
use masonry::core::{ChildrenIds, RegisterCtx};
// ---
use masonry::core::WidgetMut;
// ---
use masonry::properties::Background;
use ui_events::pointer::PointerButtonEvent;

// ---

pub struct ColorRectangle {
    size: Size,
    color: Color,
}

impl ColorRectangle {
    pub fn new(size: Size, color: Color) -> Self {
        Self { size, color }
    }
}

// ---

impl ColorRectangle {
    pub fn set_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.color = color;
        this.ctx.request_paint_only();
    }

    pub fn set_size(this: &mut WidgetMut<'_, Self>, size: Size) {
        this.widget.size = size;
        this.ctx.request_layout();
    }
}

// ---

#[derive(Debug)]
pub struct ColorRectanglePress;

impl Widget for ColorRectangle {
    type Action = ColorRectanglePress;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                ctx.submit_action::<Self::Action>(ColorRectanglePress);
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            accesskit::Action::Click => {
                ctx.submit_action::<Self::Action>(ColorRectanglePress);
            }
            _ => {}
        }
    }

    // ---

    fn on_anim_frame(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
    }
    #[cfg(false)] // We show two `update` implementations; check that both parse.
    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    // ---

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(self.size)
    }

    // ---

    #[cfg(false)] // We show two `paint` implementations; check that both parse.
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let rect = ctx.size().to_rect();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            self.color,
            Some(Affine::IDENTITY),
            &rect,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
    }

    // ---

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("ColorRectangle", id = id.trace())
    }

    // ---

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    // ---

    // Second implementation from "Creating a new widget" tutorial.
    // We use these methods in the trait, so that hovering is detected in our unit tests.

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let rect = ctx.size().to_rect();
        let color = if ctx.is_hovered() {
            Color::WHITE
        } else {
            self.color
        };
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color,
            Some(Affine::IDENTITY),
            &rect,
        );
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => {
                ctx.request_render();
            }
            _ => {}
        }
    }
}

// ---

// Implementation from "Reading widget properties" tutorial.
#[expect(dead_code, reason = "example code")]
#[expect(clippy::trivially_copy_pass_by_ref, reason = "example code")]
impl ColorRectangle {
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let background = props.get::<Background>();
        let rect = ctx.size().to_rect();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &background.get_peniko_brush_for_rect(rect),
            Some(Affine::IDENTITY),
            &rect,
        );
    }
}

// ---

#[expect(dead_code, reason = "example code")]
fn set_bg(color_rectangle_mut: WidgetMut<'_, ColorRectangle>) {
    let mut color_rectangle_mut: WidgetMut<'_, ColorRectangle> = color_rectangle_mut;

    let bg = Background::Color(masonry::palette::css::BLUE);
    color_rectangle_mut.insert_prop(bg);
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use masonry::testing::TestHarness;
    use masonry::theme::default_property_set;
    use masonry_core::core::NewWidget;
    // ---
    use masonry::testing::assert_render_snapshot;
    // ---

    const BLUE: Color = Color::from_rgb8(0, 0, u8::MAX);

    #[test]
    fn simple_rect() {
        let widget = ColorRectangle::new(Size::new(20.0, 20.0), BLUE);

        let mut harness = TestHarness::create(default_property_set(), NewWidget::new(widget));

        assert_debug_snapshot!(harness.root_widget());

        // ---

        assert_render_snapshot!(harness, "rect_blue_rectangle");
    }

    // ---

    #[test]
    fn hovered() {
        let widget = NewWidget::new(ColorRectangle::new(Size::new(20.0, 20.0), BLUE));

        let mut harness = TestHarness::create(default_property_set(), widget);
        let rect_id = harness.root_id();

        // Computes the rect's layout and sends an PointerEvent
        // placing the mouse at its center.
        harness.mouse_move_to(rect_id);
        assert_render_snapshot!(harness, "rect_hovered_rectangle");
    }

    // ---

    #[test]
    fn edit_rect() {
        const RED: Color = Color::from_rgb8(u8::MAX, 0, 0);
        let widget = NewWidget::new(ColorRectangle::new(Size::new(20.0, 20.0), BLUE));

        let mut harness = TestHarness::create(default_property_set(), widget);

        harness.edit_root_widget(|mut rect| {
            ColorRectangle::set_size(&mut rect, Size::new(50.0, 50.0));
            ColorRectangle::set_color(&mut rect, RED);
        });

        assert_render_snapshot!(harness, "rect_big_red_rectangle");
    }

    // ---

    #[test]
    fn on_click() {
        let widget = NewWidget::new(ColorRectangle::new(Size::new(20.0, 20.0), BLUE));

        let mut harness = TestHarness::create(default_property_set(), widget);
        let rect_id = harness.root_id();

        harness.mouse_click_on(rect_id);
        assert!(matches!(
            harness.pop_action::<ColorRectanglePress>(),
            Some((ColorRectanglePress, _))
        ));
    }
}
