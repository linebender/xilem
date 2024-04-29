use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{ChangeFlags, MessageResult, View, ViewCx, ViewId};

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

impl<F, State, Action> View<State, Action> for Button<F>
where
    F: Fn(&mut State) -> Action + Send + 'static,
{
    type Element = WidgetPod<masonry::widget::Button>;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCx) -> (WidgetPod<masonry::widget::Button>, Self::ViewState) {
        cx.with_leaf_action_widget(|_| {
            WidgetPod::new(masonry::widget::Button::new(self.label.clone()))
        })
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        _cx: &mut ViewCx,
        prev: &Self,
        mut element: WidgetMut<masonry::widget::Button>,
    ) -> crate::ChangeFlags {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ChangeFlags::CHANGED
        } else {
            ChangeFlags::UNCHANGED
        }
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Button::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::ButtonPressed = *action {
                    MessageResult::Action((self.callback)(app_state))
                } else {
                    tracing::error!("Wrong action type in Checkbox::message: {action:?}");
                    MessageResult::Stale(action)
                }
            }
            Err(message) => {
                tracing::error!("Wrong message type in Checkbox::message");
                MessageResult::Stale(message)
            }
        }
    }
}
