
use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::kurbo::{Point, Size};
use crate::widget::LabelText;
use crate::window_handling::win_handler::{AppHandler, AppState};
use crate::window_handling::window_description::{WindowId, WindowDesc};
use crate::{Data, Env, LocalizedString, Widget};

use druid_shell::{Application, Error as PlatformError, WindowBuilder, WindowHandle, WindowLevel};
use druid_shell::WindowState;

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher {
    windows: Vec<WindowDesc>,
    l10n_resources: Option<(Vec<String>, String)>,
    ext_event_host: ExtEventHost,
}

impl AppLauncher {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc) -> Self {
        AppLauncher {
            windows: vec![window],
            l10n_resources: None,
            ext_event_host: ExtEventHost::new(),
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

    /// Use custom localization resource
    ///
    /// `resources` is a list of file names that contain strings. `base_dir`
    /// is a path to a directory that includes per-locale subdirectories.
    ///
    /// This directory should be of the structure `base_dir/{locale}/{resource}`,
    /// where '{locale}' is a valid BCP47 language tag, and {resource} is a `.ftl`
    /// included in `resources`.
    pub fn localization_resources(mut self, resources: Vec<String>, base_dir: String) -> Self {
        self.l10n_resources = Some((resources, base_dir));
        self
    }

    /// Returns an [`ExtEventSink`] that can be moved between threads,
    /// and can be used to submit commands back to the application.
    ///
    /// [`ExtEventSink`]: struct.ExtEventSink.html
    pub fn get_external_handle(&self) -> ExtEventSink {
        self.ext_event_host.make_sink()
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(mut self) -> Result<(), PlatformError> {
        let app = Application::new()?;

        let mut env = self
            .l10n_resources
            .map(|it| Env::with_i10n(it.0, &it.1))
            .unwrap_or_else(Env::with_default_i10n);

        let mut state = AppState::new(
            app.clone(),
            env,
            self.ext_event_host,
        );

        for desc in self.windows {
            let window = desc.build_native(&mut state)?;
            window.show();
        }

        let handler = AppHandler::new(state);
        app.run(Some(Box::new(handler)));
        Ok(())
    }
}
