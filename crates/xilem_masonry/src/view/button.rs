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

impl<F, AppState, Action> MasonryView<AppState, Action> for Button<F>
where
    F: Fn(&mut AppState) -> Action + Send + 'static,
{
    type State = ();
    type Element = masonry::widget::Button;

    fn build(&self, cx: &mut ViewCx) -> (ViewId, Self::State, WidgetPod<Self::Element>) {
        let (id, element) = cx.with_action_widget::<Self, _>(|_| {
            WidgetPod::new(masonry::widget::Button::new(self.label.clone()))
        });
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut ViewCx,
        prev: &Self,
        _id: &mut ViewId,
        (): &mut Self::State,
        mut element: WidgetMut<Self::Element>,
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
        id_path: &[ViewId],
        (): &mut Self::State,
        // TODO: Ensure is masonry button pressed action?
        _message: Box<dyn std::any::Any>,
        app_state: &mut AppState,
    ) -> crate::MessageResult<Action> {
        debug_assert!(id_path.is_empty());
        crate::MessageResult::Action((self.callback)(app_state))
    }
}
