use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{ChangeFlags, MasonryView, MessageResult, ViewCx, ViewId};

pub fn checkbox<F, State, Action>(
    label: impl Into<ArcStr>,
    checked: bool,
    callback: F,
) -> Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + 'static,
{
    Checkbox {
        label: label.into(),
        callback,
        checked,
    }
}

pub struct Checkbox<F> {
    label: ArcStr,
    checked: bool,
    callback: F,
}

impl<F, State, Action> MasonryView<State, Action> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + 'static,
{
    type Element = masonry::widget::Checkbox;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        cx.with_leaf_action_widget(|_| {
            WidgetPod::new(masonry::widget::Checkbox::new(
                self.checked,
                self.label.clone(),
            ))
        })
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        _cx: &mut ViewCx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
    ) -> crate::ChangeFlags {
        let mut changeflags = ChangeFlags::UNCHANGED;
        if prev.label != self.label {
            element.set_text(self.label.clone());
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        if prev.checked != self.checked {
            element.set_checked(self.checked);
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        changeflags
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Checkbox::message"
        );
        if let Some(masonry::Action::CheckboxChecked(checked)) = message.downcast_ref() {
            return MessageResult::Action((self.callback)(app_state, *checked))
        }
        tracing::error!("Wrong message type in Checkbox::message");
        MessageResult::Stale(message)
    }

}
