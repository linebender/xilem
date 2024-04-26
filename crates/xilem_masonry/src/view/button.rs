use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{ChangeFlags, MasonryView, ViewCx, ViewId};

pub fn button<F, State, Action>(label: impl Into<ArcStr>, callback: F) -> Button<F>
where
    F: Fn(&mut State) -> Action + Send + 'static,
{
    Button {
        label: label.into(),
        callback,
    }
}

pub struct Button<F> {
    label: ArcStr,
    callback: F,
}

impl<F, State, Action> MasonryView<State, Action> for Button<F>
where
    F: Fn(&mut State) -> Action + Send + 'static,
{
    type Element = masonry::widget::Button;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        cx.with_leaf_action_widget(|_| {
            WidgetPod::new(masonry::widget::Button::new(self.label.clone()))
        })
    }
    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        // TODO: Ensure is masonry button pressed action?
        _message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        crate::MessageResult::Action((self.callback)(app_state))
    }
    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        _cx: &mut ViewCx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
    ) -> crate::ChangeFlags {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ChangeFlags::CHANGED
        } else {
            ChangeFlags::UNCHANGED
        }
    }
}
