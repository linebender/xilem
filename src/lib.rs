extern crate core;

mod app;
mod app_main;
mod bloom;
mod geometry;
mod id;
mod test_scenes;
mod text;
pub mod vg;
pub mod view;
pub mod widget;

pub use xilem_core::{IdPath, Message, MessageResult};

pub use app::App;
pub use app_main::AppLauncher;
pub(crate) use bloom::Bloom;
pub use geometry::Axis;
