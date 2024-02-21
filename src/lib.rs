extern crate core;

mod app;
mod app_main;
mod bloom;
mod geometry;
mod id;
pub mod text;
pub mod view;
pub mod widget;

xilem_core::message!(Send);

pub use xilem_core::{IdPath, MessageResult};

pub use app::App;
pub use app_main::AppLauncher;
pub(crate) use bloom::Bloom;
pub use geometry::Axis;

pub use parley;
pub use vello;
