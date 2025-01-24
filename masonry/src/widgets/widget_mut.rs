// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::contexts::MutateCtx;
use crate::{Affine, FromDynWidget, Widget};

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
/// [`EventCtx`](crate::EventCtx), [`UpdateCtx`](crate::UpdateCtx) or from a parent
/// `WidgetMut` with [`MutateCtx`].
///
/// `WidgetMut` implements [`Deref`](std::ops::Deref) with `W::Mut` as target.
///
/// ## `WidgetMut` as a Receiver
///
/// Once the Receiver trait is stabilized, `WidgetMut` will implement it so that custom
/// widgets in downstream crates can use `WidgetMut` as the receiver for inherent methods.
pub struct WidgetMut<'a, W: Widget + ?Sized> {
    pub ctx: MutateCtx<'a>,
    pub widget: &'a mut W,
}

impl<W: Widget + ?Sized> Drop for WidgetMut<'_, W> {
    fn drop(&mut self) {
        // If this `WidgetMut` is a reborrow, a parent non-reborrow `WidgetMut`
        // still exists which will do the merge-up in `Drop`.
        if let Some(parent_widget_state) = self.ctx.parent_widget_state.take() {
            parent_widget_state.merge_up(self.ctx.widget_state);
        }
    }
}

impl<W: Widget + ?Sized> WidgetMut<'_, W> {
    /// Get a `WidgetMut` for the same underlying widget with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> WidgetMut<'_, W> {
        let widget = &mut self.widget;
        WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget,
        }
    }

    /// Set the local transform of this widget.
    ///
    /// It behaves similarly as CSS transforms.
    pub fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(transform);
    }

    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn try_downcast<W2: Widget + FromDynWidget + ?Sized>(
        &mut self,
    ) -> Option<WidgetMut<'_, W2>> {
        Some(WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget: W2::from_dyn_mut(self.widget.as_mut_dyn())?,
        })
    }

    /// Downcasts to `WidgetMut` of concrete Widget type.
    ///
    /// ## Panics
    ///
    /// Panics if the downcast fails, with an error message that shows the
    /// discrepancy between the expected and actual types.
    pub fn downcast<W2: Widget + FromDynWidget + ?Sized>(&mut self) -> WidgetMut<'_, W2> {
        let w1_name = self.widget.type_name();
        match W2::from_dyn_mut(self.widget.as_mut_dyn()) {
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
