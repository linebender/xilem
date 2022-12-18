// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#[cfg(not(tarpaulin_include))]
mod win_handler;
#[cfg(not(tarpaulin_include))]
mod window_description;

pub use win_handler::{DialogInfo, MasonryAppHandler, MasonryWinHandler};
pub(crate) use win_handler::{EXT_EVENT_IDLE_TOKEN, RUN_COMMANDS_TOKEN};
pub use window_description::{WindowConfig, WindowDescription, WindowId, WindowSizePolicy};
