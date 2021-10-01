use crate::platform::WindowDesc;
use crate::platform::{AppHandler, AppState};
use crate::Env;

use druid_shell::{Application, Error as PlatformError};

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher {
    windows: Vec<WindowDesc>,
}

impl AppLauncher {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc) -> Self {
        AppLauncher {
            windows: vec![window],
        }
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

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(mut self) -> Result<(), PlatformError> {
        let app = Application::new()?;

        let mut state = AppState::new(app.clone(), Env::with_theme());

        for desc in self.windows {
            let window = desc.build_native(&mut state)?;
            window.show();
        }

        let handler = AppHandler::new(state);
        app.run(Some(Box::new(handler)));
        Ok(())
    }
}
