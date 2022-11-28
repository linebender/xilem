use glazier::kurbo::{Affine, Size};

use crate::{id::Id, widget::Pod, View, Widget};

pub fn padding<V: View<T, A>, T, A>(width: f64, view: V) -> impl View<T, A>
where
    V::Element: 'static,
{
    PaddingView::new(width, view)
}

pub struct PaddingView<V> {
    width: f64,
    view: V,
}

impl<T, A, V: View<T, A>> View<T, A> for PaddingView<V>
where
    V::Element: 'static,
{
    type State = (Id, V::State);

    type Element = PaddingWidget;

    fn build(&self, cx: &mut crate::view::Cx) -> (crate::id::Id, Self::State, Self::Element) {
        let (id, (child_id, state, element)) = cx.with_new_id(|cx| self.view.build(cx));
        let element = PaddingWidget::new(Pod::new(element), self.width);
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
        let mut changed = prev.width != self.width;
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

impl<V> PaddingView<V> {
    fn new(width: f64, view: V) -> Self {
        PaddingView { width, view }
    }
}

pub struct PaddingWidget {
    widget: Pod,
    width: f64,
}

impl PaddingWidget {
    pub fn new(widget: Pod, width: f64) -> Self {
        Self {
            widget: widget,
            width,
        }
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
        self.widget.layout(cx, proposed_size - padding_size) + padding_size
    }

    fn paint(&mut self, cx: &mut crate::widget::PaintCx, builder: &mut piet_scene::SceneBuilder) {
        self.widget.paint(cx);
        let fragment = self.widget.fragment();
        builder.append(fragment, Some(Affine::translate((self.width, self.width))))
    }
}
