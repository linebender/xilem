// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showcasing how layers can be used to implement rudimentary tooltips.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, ErasedAction, EventCtx, LayerType, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef, PropertySet,
    RegisterCtx, StyleProperty, Update, UpdateCtx, Widget, WidgetId, WidgetPod,
};
use masonry::kurbo::{Axis, Point, Size, Vec2};
use masonry::layers::Tooltip;
use masonry::layout::{LayoutSize, LenReq, SizeDef};
use masonry::parley::FontWeight;
use masonry::properties::{Background, BorderColor, BorderWidth, ContentColor};
use masonry::theme::default_property_set;
use masonry::util::{Duration, Instant};
use masonry::vello::Scene;
use masonry::vello::peniko::Color;
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

struct OverlayBox {
    child: WidgetPod<dyn Widget>,
    overlayer: Box<dyn Fn() -> (NewWidget<dyn Widget>, LayerType)>,
    layer_root_id: Option<WidgetId>,
    last_mouse_move: Option<Instant>,
    last_cursor_pos: Point,
}

// --- MARK: BUILDERS
impl OverlayBox {
    /// Construct container with child, and both width and height not set.
    fn new(
        child: NewWidget<impl Widget + ?Sized>,
        overlayer: Box<dyn Fn() -> (NewWidget<dyn Widget>, LayerType)>,
    ) -> Self {
        Self {
            child: child.erased().to_pod(),
            overlayer,
            layer_root_id: None,
            last_mouse_move: None,
            last_cursor_pos: Point::ZERO,
        }
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
        if let PointerEvent::Move(PointerUpdate { current, .. }) = event {
            self.last_cursor_pos = current.logical_point();
            self.last_mouse_move = Some(Instant::now());
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        if let Some(last_mouse_move) = self.last_mouse_move {
            let now = Instant::now();
            if now.duration_since(last_mouse_move) > Duration::from_millis(300) {
                let (overlay, layer_type) = (self.overlayer)();
                self.layer_root_id = Some(overlay.id());
                let layer_pos = self.last_cursor_pos + Vec2::new(5., -25.);
                ctx.create_layer(layer_type, overlay, layer_pos);
            } else {
                ctx.request_anim_frame();
            }
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::HoveredChanged(false) = event {
            self.last_mouse_move = None;
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        ctx.compute_length(
            &mut self.child,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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

    let overlayer = || {
        let tooltip = NewWidget::new_with_props(
            Tooltip::new(NewWidget::new_with_props(
                Label::new("Tooltip!!!"),
                PropertySet::one(ContentColor::new(Color::BLACK)),
            )),
            PropertySet::from((
                BorderWidth::all(1.),
                BorderColor::new(Color::BLACK),
                Background::Color(Color::WHITE),
            )),
        )
        .erased();
        (tooltip, LayerType::Tooltip("Tooltip!!!".to_string()))
    };

    let overlay_box = OverlayBox::new(label.with_auto_id(), Box::new(overlayer));

    // Arrange the two widgets vertically, with some padding
    let main_widget = Flex::column()
        .with_spacer(1.)
        .with_fixed(overlay_box.with_auto_id())
        .with_spacer(1.);

    let driver = Driver {};

    masonry_winit::app::run(
        vec![NewWindow::new(
            Window::default_attributes().with_title("Hello Layers!"),
            main_widget.with_auto_id().erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
