// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing how transient can be used to implement rudimentary tooltips.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use dpi::LogicalSize;
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
use masonry_winit::app::{AppDriver, AppSignal, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::{Window, WindowButtons};
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
    window_id: Option<WindowId>,
}

// --- MARK: BUILDERS
impl OverlayBox {
    /// Construct container with child.
    pub fn new<W: Widget + 'static>(
        child: NewWidget<impl Widget + ?Sized>,
        overlayer: impl Fn() -> NewWidget<W> + 'static,
    ) -> Self {
        Self {
            child: child.erased().to_pod(),
            overlayer: Box::new(move || overlayer().erased()),
            window_id: None,
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
            && self.window_id.is_none()
        {
            let overlay_root = (self.overlayer)();
            let mut position = current.logical_position();
            position.x -= 20.;
            position.y -= 12.5;
            let overlay_window = NewWindow::new(
                Window::default_attributes()
                    .with_decorations(false)
                    .with_enabled_buttons(WindowButtons::empty())
                    .with_inner_size(LogicalSize::new(40., 10.))
                    .with_active(false)
                    .with_position(position),
                overlay_root,
            );
            self.window_id = Some(overlay_window.id);
            ctx.submit_app_signal(AppSignal::CreateWindow(overlay_window));
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::HoveredChanged(false) = event
            && let Some(overlay_id) = self.window_id.take()
        {
            ctx.submit_app_signal(AppSignal::CloseWindow(overlay_id));
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

    let overlayer = || Label::new("Tooltip!!!").with_auto_id();

    let overlay_box = OverlayBox::new(label.with_auto_id(), Box::new(overlayer));

    // Arrange the two widgets vertically, with some padding
    let main_widget = Flex::column()
        .with_flex_spacer(1.)
        .with_child(overlay_box.with_auto_id())
        .with_flex_spacer(1.);

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
