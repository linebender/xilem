use std::ops::{Deref, DerefMut};

use crate::widget::StoreInWidgetMut;
use crate::{Widget, WidgetCtx, WidgetId, WidgetState};

// TODO - rename lifetimes
pub struct WidgetMut<'a, 'b: 'a, W: Widget + StoreInWidgetMut> {
    pub(crate) parent_widget_state: &'a mut WidgetState,
    pub(crate) inner: W::Mut<'a, 'b>,
}

impl<W: StoreInWidgetMut> Drop for WidgetMut<'_, '_, W> {
    fn drop(&mut self) {
        self.parent_widget_state
            .merge_up(W::get_ctx(&mut self.inner).widget_state);
    }
}

// --- Ref logic ---

impl<'a, 'b, W: StoreInWidgetMut> Deref for WidgetMut<'a, 'b, W> {
    type Target = W::Mut<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'b, W: StoreInWidgetMut> DerefMut for WidgetMut<'a, 'b, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, 'b, W: StoreInWidgetMut> WidgetMut<'a, 'b, W> {
    pub fn downcast<'s, W2: Widget + StoreInWidgetMut>(
        &'s mut self,
    ) -> Option<WidgetMut<'_, 'b, W2>> {
        let (widget, ctx) = W::get_widget_and_ctx(&mut self.inner);
        let widget = widget.as_mut_any().downcast_mut()?;
        let ctx = WidgetCtx {
            global_state: ctx.global_state,
            widget_state: ctx.widget_state,
            is_init: ctx.is_init,
        };
        Some(WidgetMut {
            parent_widget_state: self.parent_widget_state,
            inner: W2::from_widget_and_ctx(widget, ctx),
        })
    }

    pub fn state(&mut self) -> &WidgetState {
        &W::get_ctx(&mut self.inner).widget_state
    }

    /// get the `WidgetId` of the current widget.
    pub fn id(&mut self) -> WidgetId {
        W::get_ctx(&mut self.inner).widget_state.id
    }
}

// TODO - unit tests
