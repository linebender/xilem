// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::event_loop_runner::MasonryState;
use crate::{Action, RenderRoot, WidgetId};

pub struct DriverCtx<'a> {
    // TODO
    // This is exposed publicly for now to let people drive
    // masonry on their own, but this is not expected to be
    // stable or even supported. This is for short term
    // expedience only while better solutions are devised.
    #[doc(hidden)]
    pub render_root: &'a mut RenderRoot,
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

    /// Return a reference to the [`RenderRoot`].
    pub fn render_root(&mut self) -> &mut RenderRoot {
        self.render_root
    }

    pub fn content_changed(&self) -> bool {
        self.render_root.needs_rewrite_passes()
    }
}
