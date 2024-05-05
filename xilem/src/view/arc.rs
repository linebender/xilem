use std::{any::Any, ops::Deref, sync::Arc};

use masonry::widget::WidgetMut;

use crate::{MasonryView, MessageResult, ViewCx, ViewId};

impl<State: 'static, Action: 'static, V: MasonryView<State, Action>> MasonryView<State, Action>
    for Arc<V>
{
    type ViewState = V::ViewState;

    type Element = V::Element;

    fn build(&self, cx: &mut ViewCx) -> (masonry::WidgetPod<Self::Element>, Self::ViewState) {
        self.deref().build(cx)
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut ViewCx,
        prev: &Self,
        element: WidgetMut<Self::Element>,
    ) {
        if !Arc::ptr_eq(self, prev) {
            self.deref().rebuild(view_state, cx, prev.deref(), element);
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.deref()
            .message(view_state, id_path, message, app_state)
    }
}
