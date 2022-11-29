use glazier::kurbo::Affine;
use piet_scene::{Color, Fill};

use crate::{id::Id, widget::Pod, View, Widget};

pub struct BackgroundWidget {
    pub(crate) widget: Pod,
    color: Color,
}

impl BackgroundWidget {
    pub fn new(widget: Pod, color: Color) -> Self {
        Self { widget, color }
    }
    pub(crate) fn set_color(&mut self, color: Color) {
        self.color = color;
    }
}

impl Widget for BackgroundWidget {
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
        self.widget.update(cx)
    }

    fn measure(
        &mut self,
        cx: &mut crate::widget::LayoutCx,
    ) -> (glazier::kurbo::Size, glazier::kurbo::Size) {
        self.widget.measure(cx)
    }

    fn layout(
        &mut self,
        cx: &mut crate::widget::LayoutCx,
        proposed_size: glazier::kurbo::Size,
    ) -> glazier::kurbo::Size {
        self.widget.layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut crate::widget::PaintCx, builder: &mut piet_scene::SceneBuilder) {
        self.widget.paint(cx);
        let fragment = self.widget.fragment();
        builder.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            self.color,
            None,
            &cx.size().to_rect(),
        );
        builder.append(fragment, None)
    }
}
