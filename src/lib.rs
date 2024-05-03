// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

extern crate core;

mod app;
mod app_main;
mod geometry;
mod id;
pub mod text;
pub mod view;
pub mod widget;

xilem_core::message!(Send);

pub use xilem_core::{IdPath, MessageResult};

pub use app::App;
pub use app_main::AppLauncher;
pub use geometry::Axis;

pub use parley;
pub use vello;
