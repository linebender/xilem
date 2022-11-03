use std::ops::{Deref, DerefMut};

use crate::widget::StoreInWidgetMut;
use crate::{Widget, WidgetCtx, WidgetId, WidgetState};

/// A mutable reference to a [`Widget`].
///
/// In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut`
/// wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`.
/// This helps Masonry make sure that internal metadata is propagated after every widget
/// change.
///
/// You can create a `WidgetMut` from [`TestHarness`](crate::testing::TestHarness),
/// [`EventCtx`](crate::EventCtx), [`LifeCycleCtx`](crate::LifeCycleCtx) or from a parent
/// `WidgetMut` with [`WidgetCtx`](crate::WidgetCtx).
///
/// `WidgetMut` implements [`Deref`] with `W::Mut` as target.
///
/// ## Internals
///
/// `WidgetMut<W>` requires that W implement [`StoreInWidgetMut`]; it stores a `W::Mut`,
/// which is a special type declared with the `declare_widget` macro. Methods to mutate
/// the widget will be implemented in that `W::Mut` type, which `WidgetMut<W>` derefs to.
///
/// See [`declare_widget`](crate::declare_widget) for details.
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

impl<'a, 'b> WidgetMut<'a, 'b, Box<dyn Widget>> {
    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn downcast<'s, W2: Widget + StoreInWidgetMut>(
        &'s mut self,
    ) -> Option<WidgetMut<'_, 'b, W2>> {
        let (widget, ctx) = Box::<dyn Widget>::get_widget_and_ctx(&mut self.inner);
        let widget = widget.as_mut_any().downcast_mut()?;
        let ctx = WidgetCtx {
            global_state: ctx.global_state,
            widget_state: ctx.widget_state,
        };
        Some(WidgetMut {
            parent_widget_state: self.parent_widget_state,
            inner: W2::from_widget_and_ctx(widget, ctx),
        })
    }
}

impl<W: StoreInWidgetMut> WidgetMut<'_, '_, W> {
    /// Get the [`WidgetState`] of the current widget.
    pub fn state(&mut self) -> &WidgetState {
        &W::get_ctx(&mut self.inner).widget_state
    }

    /// Get the [`WidgetId`] of the current widget.
    pub fn id(&mut self) -> WidgetId {
        W::get_ctx(&mut self.inner).widget_state.id
    }
}

// TODO - unit tests
