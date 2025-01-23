// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs, reason = "WIP")]

pub(crate) mod app_driver;
pub mod event_loop_runner;
pub(crate) mod render_root;

pub(crate) mod tracing_backend;

pub use app_driver::{AppDriver, DriverCtx};
pub use event_loop_runner::{
    run, run_with, EventLoop, EventLoopBuilder, EventLoopProxy, MasonryState, MasonryUserEvent,
};
pub use render_root::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};

pub(crate) use render_root::{MutateCallback, RenderRootState};
pub(crate) use tracing_backend::{try_init_test_tracing, try_init_tracing};
