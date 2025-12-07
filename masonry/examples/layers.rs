// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing how layers can be used to implement rudimentary tooltips.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, BoxConstraints, ChildrenIds, ErasedAction, EventCtx, LayoutCtx, NewWidget, NoAction,
    PaintCtx, PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef, RegisterCtx,
    StyleProperty, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use masonry::kurbo::{Point, Size};
use masonry::parley::FontWeight;
use masonry::theme::default_property_set;
use masonry::vello::Scene;
use masonry::widgets::{Flex, Label};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;
use tracing::{Span, trace_span};

struct Driver;

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        _window_id: WindowId,
        _ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        _action: ErasedAction,
    ) {
    }
}

#[allow(missing_docs, missing_debug_implementations, reason = "example code")]
pub struct OverlayBox {
    child: WidgetPod<dyn Widget>,
    overlayer: Box<dyn Fn() -> NewWidget<dyn Widget>>,
    layer_root_id: Option<WidgetId>,
}

// --- MARK: BUILDERS
impl OverlayBox {
    /// Construct container with child, and both width and height not set.
    pub fn new(
        child: NewWidget<impl Widget + ?Sized>,
        overlayer: Box<dyn Fn() -> NewWidget<dyn Widget>>,
    ) -> Self {
        Self {
            child: child.erased().to_pod(),
            overlayer,
            layer_root_id: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl OverlayBox {
    /// Get mutable reference to the child widget, if any.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

// --- MARK: IMPL WIDGET
impl Widget for OverlayBox {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if let PointerEvent::Move(PointerUpdate { current, .. }) = event
            && ctx.is_hovered()
        {
            let position = current.logical_point();
            if let Some(overlay_id) = self.layer_root_id {
                ctx.reposition_layer(overlay_id, position);
            } else {
                let overlay = (self.overlayer)();
                self.layer_root_id = Some(overlay.id());
                ctx.create_layer(overlay, position);
            }
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::HoveredChanged(false) = event
            && let Some(overlay_id) = self.layer_root_id.take()
        {
            ctx.remove_layer(overlay_id);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("OverlayBox", id = id.trace())
    }
}

fn main() {
    let label = Label::new("Hello")
        .with_style(StyleProperty::FontSize(32.0))
        // Ideally there's be an Into in Parley for this
        .with_style(StyleProperty::FontWeight(FontWeight::BOLD));

    let overlayer = || Label::new("Tooltip!!!").with_auto_id().erased();

    let overlay_box = OverlayBox::new(label.with_auto_id(), Box::new(overlayer));

    // Arrange the two widgets vertically, with some padding
    let main_widget = Flex::column()
        .with_spacer(1.)
        .with_fixed(overlay_box.with_auto_id())
        .with_spacer(1.);

    let driver = Driver {};

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![NewWindow::new(
            Window::default_attributes().with_title("Hello Layers!"),
            main_widget.with_auto_id().erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
