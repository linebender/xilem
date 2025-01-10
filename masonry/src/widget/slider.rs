use accesskit::{Node, Role};
use smallvec::SmallVec;
use vello::peniko::Color;
use vello::{kurbo::Circle, Scene};

use crate::contexts::{AccessCtx, ComposeCtx, EventCtx, LayoutCtx, PaintCtx, RegisterCtx, UpdateCtx};
use crate::event::{PointerEvent, Update};
use crate::{AllowRawMut, BoxConstraints, Point, Rect, Size, Widget, WidgetId, WidgetPod};

/// A slider widget for selecting a value within a range.
pub struct Slider {
    min: f64,
    max: f64,
    value: f64,
    on_change: Box<dyn Fn(f64)>,
    thumb_radius: f64,
    track_height: f64,
    thumb_color: Color,
    track_color: Color,
    child: WidgetPod<Box<dyn Widget>>,
}

impl Slider {
    /// Create a new slider with the given range and initial value.
    pub fn new(min: f64, max: f64, value: f64, on_change: impl Fn(f64) + 'static) -> Self {
        Self {
            min,
            max,
            value: value.clamp(min, max),
            on_change: Box::new(on_change),
            thumb_radius: 8.0,
            track_height: 4.0,
            thumb_color: Color::rgb8(0, 122, 255),
            track_color: Color::rgb8(200, 200, 200),
            child: WidgetPod::new(Box::new(())),
        }
    }

    /// Set the thumb radius.
    pub fn thumb_radius(mut self, radius: f64) -> Self {
        self.thumb_radius = radius;
        self
    }

    /// Set the track height.
    pub fn track_height(mut self, height: f64) -> Self {
        self.track_height = height;
        self
    }

    /// Set the thumb color.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Set the track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    fn value_from_x(&self, x: f64, width: f64) -> f64 {
        let t = (x / width).clamp(0.0, 1.0);
        self.min + t * (self.max - self.min)
    }
}

impl Widget for Slider {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(pos) => {
                let width = ctx.size().width;
                let new_value = self.value_from_x(pos.x, width);
                if (new_value - self.value).abs() > f64::EPSILON {
                    self.value = new_value;
                    (self.on_change)(self.value);
                    ctx.request_paint();
                }
                ctx.capture_pointer();
            }
            PointerEvent::PointerMove(pos) if ctx.has_pointer_capture() => {
                let width = ctx.size().width;
                let new_value = self.value_from_x(pos.x, width);
                if (new_value - self.value).abs() > f64::EPSILON {
                    self.value = new_value;
                    (self.on_change)(self.value);
                    ctx.request_paint();
                }
            }
            PointerEvent::PointerUp(_) => {
                ctx.release_pointer();
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let width = bc.max().width;
        let height = self.thumb_radius * 2.0;
        Size::new(width, height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();
        let width = size.width;
        let height = size.height;

        // Draw track
        let track_rect = Rect::new(
            0.0,
            (height - self.track_height) / 2.0,
            width,
            (height + self.track_height) / 2.0,
        );
        scene.fill(
            track_rect,
            &self.track_color,
            None,
            Affine::IDENTITY,
            None,
        );

        // Draw thumb
        let t = (self.value - self.min) / (self.max - self.min);
        let thumb_x = t * width;
        let thumb_y = height / 2.0;
        scene.fill(
            Circle::new(Point::new(thumb_x, thumb_y), self.thumb_radius),
            &self.thumb_color,
            None,
            Affine::IDENTITY,
            None,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) {
        node.set_min_value(Some(self.min));
        node.set_max_value(Some(self.max));
        node.set_value(Some(self.value));
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        self.child.update(ctx, event);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx) {
        self.child.compose(ctx);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        let mut ids = SmallVec::new();
        ids.push(self.child.id());
        ids
    }
}

impl AllowRawMut for Slider {}
