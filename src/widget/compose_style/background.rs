use glazier::kurbo::Affine;
use piet_scene::{Color, Fill};

use crate::{id::Id, widget::Pod, View, Widget};

pub fn background<V: View<T, A>, T, A>(color: Color, view: V) -> impl View<T, A>
where
    V::Element: 'static,
{
    BackgroundView::new(color, view)
}

pub struct BackgroundView<V> {
    color: Color,
    view: V,
}

impl<T, A, V: View<T, A>> View<T, A> for BackgroundView<V>
where
    V::Element: 'static,
{
    type State = (Id, V::State);

    type Element = BackgroundWidget;

    fn build(&self, cx: &mut crate::view::Cx) -> (crate::id::Id, Self::State, Self::Element) {
        let (id, (child_id, state, element)) = cx.with_new_id(|cx| self.view.build(cx));
        let element = BackgroundWidget::new(Pod::new(element), self.color);
        (id, (child_id, state), element)
    }

    fn rebuild(
        &self,
        cx: &mut crate::view::Cx,
        prev: &Self,
        id: &mut crate::id::Id,
        (child_id, state): &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        let mut changed = prev.color != self.color;
        cx.with_id(*id, |cx| {
            let child_element = element.widget.downcast_mut().unwrap();
            changed |= self
                .view
                .rebuild(cx, &prev.view, child_id, state, child_element)
        });
        changed
    }

    fn event(
        &self,
        id_path: &[crate::id::Id],
        (child_id, state): &mut Self::State,
        event: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::event::EventResult<A> {
        let (left, right) = id_path.split_at(1);
        assert!(left[0] == *child_id);
        self.view.event(right, state, event, app_state)
    }
}

impl<V> BackgroundView<V> {
    fn new(color: Color, view: V) -> Self {
        BackgroundView { color, view }
    }
}

pub struct BackgroundWidget {
    widget: Pod,
    color: Color,
}

impl BackgroundWidget {
    pub fn new(widget: Pod, color: Color) -> Self {
        Self { widget, color }
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
