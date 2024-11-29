// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::event_loop_runner::MasonryState;
use crate::widget::WidgetMut;
use crate::{Action, Widget, WidgetId};

pub struct DriverCtx<'a> {
    // TODO
    // This is exposed publicly for now to let people drive
    // masonry on their own, but this is not expected to be
    // stable or even supported. This is for short term
    // expedience only while better solutions are devised.
    #[doc(hidden)]
    pub main_root_widget: WidgetMut<'a, Box<dyn Widget>>,
}

pub trait AppDriver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action);

    #[allow(unused_variables)]
    // reason: otherwise `state` would need to be named `_state` which behaves badly when using rust-analyzer to implement the trait
    /// A hook which will be executed when the application starts, to allow initial configuration of the `MasonryState`.
    ///
    /// Use cases include loading fonts.
    fn on_start(&mut self, state: &mut MasonryState) {}
}

impl DriverCtx<'_> {
    // TODO - Add method to create timer

    /// Return a [`WidgetMut`] to the root widget.
    pub fn get_root<W: Widget>(&mut self) -> WidgetMut<'_, W> {
        self.main_root_widget.downcast()
    }

    pub fn content_changed(&self) -> bool {
        let ctx = &self.main_root_widget.ctx;
        ctx.widget_state.needs_rewrite_passes()
            || ctx.widget_state.needs_render()
            || ctx.global_state.focus_changed()
    }
}
