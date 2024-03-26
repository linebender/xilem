// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use druid_shell::{Application as AppHandle, Error as PlatformError};

use crate::app_delegate::AppDelegate;
use crate::app_root::AppRoot;
use crate::ext_event::{ExtEventQueue, ExtEventSink};
use crate::platform::{MasonryAppHandler, WindowDescription};

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher {
    windows: Vec<WindowDescription>,
    app_delegate: Option<Box<dyn AppDelegate>>,
    ext_event_queue: ExtEventQueue,
}

impl AppLauncher {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDescription) -> Self {
        AppLauncher {
            windows: vec![window],
            app_delegate: None,
            ext_event_queue: ExtEventQueue::new(),
        }
    }

    /// Set the [`AppDelegate`].
    ///
    /// [`AppDelegate`]: trait.AppDelegate.html
    pub fn with_delegate(mut self, delegate: impl AppDelegate + 'static) -> Self {
        self.app_delegate = Some(Box::new(delegate));
        self
    }

    /// Initialize a minimal tracing subscriber with DEBUG max level for printing logs out to
    /// stderr.
    ///
    /// This is meant for quick-and-dirty debugging. If you want more serious trace handling,
    /// it's probably better to implement it yourself.
    ///
    /// # Panics
    ///
    /// Panics if the subscriber fails to initialize.
    pub fn log_to_console(self) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use tracing_subscriber::prelude::*;
            let filter_layer = tracing_subscriber::filter::LevelFilter::DEBUG;
            let fmt_layer = tracing_subscriber::fmt::layer()
                // Display target (eg "my_crate::some_mod::submod") with logs
                .with_target(true);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }
        // Note - tracing-wasm might not work in headless Node.js. Probably doesn't matter anyway,
        // because this is a GUI framework, so wasm targets will virtually always be browsers.
        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();
            let config = tracing_wasm::WASMLayerConfigBuilder::new()
                .set_max_level(tracing::Level::DEBUG)
                .build();
            tracing_wasm::set_as_global_default_with_config(config)
        }
        self
    }

    /// Returns an [`ExtEventSink`] that can be moved between threads,
    /// and can be used to submit commands back to the application.
    pub fn get_external_handle(&self) -> ExtEventSink {
        self.ext_event_queue.make_sink()
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(self) -> Result<(), PlatformError> {
        let app = AppHandle::new()?;
        let state = AppRoot::create(
            app.clone(),
            self.windows,
            self.app_delegate,
            self.ext_event_queue,
        )?;
        let handler = MasonryAppHandler::new(state);

        app.run(Some(Box::new(handler)));
        Ok(())
    }
}
