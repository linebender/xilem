// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::contexts::MutateCtx;
use crate::Widget;

// TODO - Document extension trait workaround.
// See https://xi.zulipchat.com/#narrow/stream/317477-masonry/topic/Thoughts.20on.20simplifying.20WidgetMut/near/436478885
/// A mutable reference to a [`Widget`].
///
/// In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut`
/// wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`.
/// This helps Masonry make sure that internal metadata is propagated after every widget
/// change.
///
/// You can create a `WidgetMut` from [`TestHarness`](crate::testing::TestHarness),
/// [`EventCtx`](crate::EventCtx), [`LifeCycleCtx`](crate::LifeCycleCtx) or from a parent
/// `WidgetMut` with [`MutateCtx`].
///
/// `WidgetMut` implements [`Deref`](std::ops::Deref) with `W::Mut` as target.
///
/// ## `WidgetMut` as a Receiver
///
/// Once the Receiver trait is stabilized, `WidgetMut` will implement it so that custom
/// widgets in downstream crates can use `WidgetMut` as the receiver for inherent methods.
pub struct WidgetMut<'a, W: Widget> {
    pub ctx: MutateCtx<'a>,
    pub widget: &'a mut W,
}

impl<W: Widget> Drop for WidgetMut<'_, W> {
    fn drop(&mut self) {
        // If this `WidgetMut` is a reborrow, a parent non-reborrow `WidgetMut`
        // still exists which will do the merge-up in `Drop`.
        if let Some(parent_widget_state) = self.ctx.parent_widget_state.take() {
            parent_widget_state.merge_up(self.ctx.widget_state);
        }
    }
}

impl<'w, W: Widget> WidgetMut<'w, W> {
    /// Get a `WidgetMut` for the same underlying widget with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> WidgetMut<'_, W> {
        let widget = &mut self.widget;
        WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget,
        }
    }
}

impl<'a> WidgetMut<'a, Box<dyn Widget>> {
    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn try_downcast<W2: Widget>(&mut self) -> Option<WidgetMut<'_, W2>> {
        Some(WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget: self.widget.as_mut_any().downcast_mut()?,
        })
    }

    /// Downcasts to `WidgetMut` of concrete Widget type.
    ///
    /// ## Panics
    ///
    /// Panics if the downcast fails, with an error message that shows the
    /// discrepancy between the expected and actual types.
    pub fn downcast<W2: Widget>(&mut self) -> WidgetMut<'_, W2> {
        let w1_name = self.widget.type_name();
        match self.widget.as_mut_any().downcast_mut() {
            Some(widget) => WidgetMut {
                ctx: self.ctx.reborrow_mut(),
                widget,
            },
            None => {
                panic!(
                    "failed to downcast widget: expected widget of type `{}`, found `{}`",
                    std::any::type_name::<W2>(),
                    w1_name,
                );
            }
        }
    }
}

// TODO - unit tests
