// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::contexts::WidgetCtx;
use crate::{Widget, WidgetState};

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
/// `WidgetMut` with [`WidgetCtx`](crate::WidgetCtx).
///
/// `WidgetMut` implements [`Deref`](std::ops::Deref) with `W::Mut` as target.
///
/// ## `WidgetMut` as a Receiver
///
/// Once the Receiver trait is stabilized, `WidgetMut` will implement it so that custom
/// widgets in downstream crates can use `WidgetMut` as the receiver for inherent methods.
pub struct WidgetMut<'a, W: Widget> {
    pub ctx: WidgetCtx<'a>,
    pub widget: &'a mut W,
}

impl<W: Widget> Drop for WidgetMut<'_, W> {
    fn drop(&mut self) {
        self.ctx.parent_widget_state.merge_up(self.ctx.widget_state);
    }
}

impl<'w, W: Widget> WidgetMut<'w, W> {
    // TODO - Replace with individual methods from WidgetState
    /// Get the [`WidgetState`] of the current widget.
    pub fn state(&self) -> &WidgetState {
        self.ctx.widget_state
    }
}

impl<'a> WidgetMut<'a, Box<dyn Widget>> {
    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn try_downcast<W2: Widget>(&mut self) -> Option<WidgetMut<'_, W2>> {
        let ctx = WidgetCtx {
            global_state: self.ctx.global_state,
            parent_widget_state: self.ctx.parent_widget_state,
            widget_state: self.ctx.widget_state,
            widget_state_children: self.ctx.widget_state_children.reborrow_mut(),
            widget_children: self.ctx.widget_children.reborrow_mut(),
        };
        Some(WidgetMut {
            ctx,
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
        let ctx = WidgetCtx {
            global_state: self.ctx.global_state,
            parent_widget_state: self.ctx.parent_widget_state,
            widget_state: self.ctx.widget_state,
            widget_state_children: self.ctx.widget_state_children.reborrow_mut(),
            widget_children: self.ctx.widget_children.reborrow_mut(),
        };
        let w1_name = self.widget.type_name();
        match self.widget.as_mut_any().downcast_mut() {
            Some(widget) => WidgetMut { ctx, widget },
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
