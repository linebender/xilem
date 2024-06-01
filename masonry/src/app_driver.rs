// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::widget::WidgetMut;
use crate::{Action, Widget, WidgetId};

// xilem::App will implement AppDriver

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
}

impl<'a> DriverCtx<'a> {
    /// Return a [`WidgetMut`] to the root widget.
    pub fn get_root<W: Widget>(&mut self) -> WidgetMut<'_, W> {
        self.main_root_widget.downcast()
    }
}
