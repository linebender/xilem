use glazier::kurbo::{Affine, Point, Size};

use crate::{widget::Pod, View, Widget};

pub struct PaddingWidget {
    pub(crate) widget: Pod,
    width: f64,
}

impl PaddingWidget {
    pub fn new(widget: Pod, width: f64) -> Self {
        Self { widget, width }
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }
}

impl Widget for PaddingWidget {
    fn event(&mut self, cx: &mut crate::widget::EventCx, event: &crate::widget::RawEvent) {
        self.widget.event(cx, event)
    }

    fn lifecycle(
        &mut self,
        cx: &mut crate::widget::contexts::LifeCycleCx,
        event: &crate::widget::LifeCycle,
    ) {
        self.widget.lifecycle(cx, event)
    }

    fn update(&mut self, cx: &mut crate::widget::UpdateCx) {
        cx.request_layout();
        self.widget.update(cx)
    }

    fn measure(
        &mut self,
        cx: &mut crate::widget::LayoutCx,
    ) -> (glazier::kurbo::Size, glazier::kurbo::Size) {
        let (min, max) = self.widget.measure(cx);
        let size = Size::new(self.width * 2., self.width * 2.);
        (min + size, max + size)
    }

    fn layout(
        &mut self,
        cx: &mut crate::widget::LayoutCx,
        proposed_size: glazier::kurbo::Size,
    ) -> glazier::kurbo::Size {
        let padding_size = Size::new(self.width * 2., self.width * 2.);
        let result = self.widget.layout(cx, proposed_size - padding_size) + padding_size;
        self.widget.state.origin = Point {
            x: self.width,
            y: self.width,
        };
        result
    }

    fn paint(&mut self, cx: &mut crate::widget::PaintCx, builder: &mut piet_scene::SceneBuilder) {
        self.widget.paint(cx);
        let fragment = self.widget.fragment();
        builder.append(fragment, Some(Affine::translate((self.width, self.width))))
    }
}
