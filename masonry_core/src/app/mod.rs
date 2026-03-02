// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types needed for running a Masonry app.

mod layer_stack;
mod render_root;
mod tracing_backend;

pub use render_root::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};

// Re-export paint result types for consumers of `RenderRoot::redraw()`.
pub use crate::passes::paint::{PaintResult, PaintedLayer};
pub use tracing_backend::{
    TracingSubscriberHasBeenSetError, default_tracing_subscriber, try_init_test_tracing,
    try_init_tracing,
};

pub(crate) use render_root::{MutateCallback, RenderRootState};
