use crate::widget::{StoreInWidgetMut, WidgetMut};
use crate::{Action, Widget, WidgetId};

// xilem::App will implement AppDriver

pub struct DriverCtx<'a> {
    // TODO
    pub(crate) main_root_widget: WidgetMut<'a, Box<dyn Widget>>,
}

pub trait AppDriver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action);
}

impl<'a> DriverCtx<'a> {
    /// Return a [`WidgetMut`] to the root widget.
    pub fn get_root<W: Widget + StoreInWidgetMut>(&mut self) -> WidgetMut<'_, W> {
        self.main_root_widget.downcast().expect("wrong widget type")
    }
}
