use piet_scene::Color;

use crate::{
    id::Id,
    widget::{
        compose_style::{background::BackgroundWidget, padding::PaddingWidget},
        Pod,
    },
    View,
};

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
        if changed {
            element.set_width(self.width);
        }
        cx.with_id(*id, |cx| {
            let child_element = element.widget.downcast_mut().unwrap();
            let child_changed = self
                .view
                .rebuild(cx, &prev.view, child_id, state, child_element);
            if child_changed {
                changed = true;
                element.widget.request_update();
            }
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
        if changed {
            element.set_color(self.color);
        }
        cx.with_id(*id, |cx| {
            let child_element = element.widget.downcast_mut().unwrap();
            let child_changed = self
                .view
                .rebuild(cx, &prev.view, child_id, state, child_element);
            if child_changed {
                changed = true;
                element.widget.request_update();
            }
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
