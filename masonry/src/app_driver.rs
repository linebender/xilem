// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::event_loop_runner::MasonryState;
use crate::{Action, RenderRoot, WidgetId};

/// Context for the [`AppDriver`] trait.
///
/// Currently holds a reference to the [`RenderRoot`].
pub struct DriverCtx<'a> {
    // We make no guarantees about the fields of this struct, but
    // they must all be public so that the type can be constructed
    // externally.
    // This is needed for external users, whilst our external API
    // is not yet designed.
    #[doc(hidden)]
    pub render_root: &'a mut RenderRoot,
}

/// A trait for defining how your app interacts with the Masonry widget tree.
///
/// When launching your app with [`crate::event_loop_runner::run`], you need to provide
/// a type that implements this trait.
pub trait AppDriver {
    /// A hook which will be executed when a widget emits an [`Action`].
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

    /// Access the [`RenderRoot`].
    pub fn render_root(&mut self) -> &mut RenderRoot {
        self.render_root
    }

    /// Returns true if something happened that requires a rewrite pass or a re-render.
    pub fn content_changed(&self) -> bool {
        self.render_root.needs_rewrite_passes()
    }
}

#[cfg(doctest)]
/// Doctests aren't collected under `cfg(test)`; we can use `cfg(doctest)` instead
mod doctests {
    /// ```no_run
    /// use masonry::DriverCtx;
    /// let _ctx = DriverCtx {
    ///     render_root: unimplemented!()
    /// };
    /// ```
    const _DRIVER_CTX_EXTERNALLY_CONSTRUCTIBLE: () = {};
}
