// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types needed for running a Masonry app.

mod render_root;
mod tracing_backend;

pub use render_root::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
pub use tracing_backend::{
    TracingSubscriberHasBeenSetError, default_tracing_subscriber, try_init_test_tracing,
    try_init_tracing,
};

pub(crate) use render_root::{MutateCallback, RenderRootState};
