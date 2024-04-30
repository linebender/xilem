// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::contexts::WidgetCtx;
use crate::widget::StoreInWidgetMut;
use crate::{Widget, WidgetId, WidgetState};

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
pub struct WidgetMut<'a, W: Widget + StoreInWidgetMut> {
    pub(crate) parent_widget_state: ManuallyDrop<&'a mut WidgetState>,
    pub(crate) inner: ManuallyDrop<W::Mut<'a>>,
    /// Whether this `WidgetMut` is the "root" `WidgetMut` of the reborrow tree, i.e.
    /// whether we need to call merge_up in drop, or whether that will happen later
    root: bool,
}

impl<'a, W: Widget + StoreInWidgetMut> WidgetMut<'a, W> {
    pub(crate) fn new(parent_widget_state: &'a mut WidgetState, inner: W::Mut<'a>) -> Self {
        Self {
            parent_widget_state: ManuallyDrop::new(parent_widget_state),
            inner: ManuallyDrop::new(inner),
            root: true,
        }
    }

    fn new_reborrowed(parent_widget_state: &'a mut WidgetState, inner: W::Mut<'a>) -> Self {
        Self {
            parent_widget_state: ManuallyDrop::new(parent_widget_state),
            inner: ManuallyDrop::new(inner),
            root: false,
        }
    }
}

impl<'a, W: StoreInWidgetMut> WidgetMut<'a, W> {
    pub fn reborrow(&mut self) -> WidgetMut<'_, W> {
        WidgetMut::new_reborrowed(&mut self.parent_widget_state, W::reborrow(&mut self.inner))
    }
}

impl<W: StoreInWidgetMut> Drop for WidgetMut<'_, W> {
    fn drop(&mut self) {
        if self.root {
            self.parent_widget_state
                .merge_up(W::get_ctx(&mut self.inner).widget_state);
        }
        #[allow(unsafe_code)]
        unsafe {
            // Safety: We never call this drop whilst self is in an invalid state
            ManuallyDrop::drop(&mut self.inner);
        }
    }
}

// --- Ref logic ---

impl<'a, W: StoreInWidgetMut> Deref for WidgetMut<'a, W> {
    type Target = W::Mut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, W: StoreInWidgetMut> DerefMut for WidgetMut<'a, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a> WidgetMut<'a, Box<dyn Widget>> {
    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn downcast<W2: Widget + StoreInWidgetMut>(&mut self) -> Option<WidgetMut<'_, W2>> {
        let (widget, ctx) = Box::<dyn Widget>::get_widget_and_ctx(&mut self.inner);
        let widget = widget.as_mut_any().downcast_mut()?;
        let ctx = WidgetCtx {
            global_state: ctx.global_state,
            widget_state: ctx.widget_state,
        };
        Some(WidgetMut::new_reborrowed(
            &mut self.parent_widget_state,
            W2::from_widget_and_ctx(widget, ctx),
        ))
    }

    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn downcast_owned<W2: Widget + StoreInWidgetMut>(mut self) -> Option<WidgetMut<'a, W2>> {
        #![allow(unsafe_code)]
        let root = self.root;
        // Safety: We run no possible panicking code between the ManuallyDrop::take sand forgetting self
        let parent_widget_state = unsafe { ManuallyDrop::take(&mut self.parent_widget_state) };
        let widget_mut = unsafe { ManuallyDrop::take(&mut self.inner) };
        // Logic: The merge_up is definitely called, either in the None
        // arm below or in the destructor of the returned value
        std::mem::forget(self);
        let (widget, ctx) = Box::<dyn Widget>::into_widget_and_ctx(widget_mut);
        match widget.as_mut_any().downcast_mut() {
            Some(widget) => {
                let ctx = WidgetCtx {
                    global_state: ctx.global_state,
                    widget_state: ctx.widget_state,
                };
                let mut widget_mut =
                    WidgetMut::new(parent_widget_state, W2::from_widget_and_ctx(widget, ctx));
                widget_mut.root = root;
                Some(widget_mut)
            }
            None => {
                if root {
                    parent_widget_state.merge_up(ctx.widget_state);
                }
                None
            }
        }
    }
}

impl<W: StoreInWidgetMut> WidgetMut<'_, W> {
    /// Get the [`WidgetState`] of the current widget.
    pub fn state(&mut self) -> &WidgetState {
        W::get_ctx(&mut self.inner).widget_state
    }

    /// Get the [`WidgetId`] of the current widget.
    pub fn id(&mut self) -> WidgetId {
        W::get_ctx(&mut self.inner).widget_state.id
    }
}

// TODO - unit tests
