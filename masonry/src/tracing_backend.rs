// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![warn(rustdoc::broken_intra_doc_links, clippy::doc_markdown, missing_docs)]

//! Configures a suitable default [`tracing`] implementation for a Masonry application.
//!
//! This uses a custom log format specialised for GUI applications,
//! and will write all logs to a temporary file in debug mode.
//! This also uses a default filter, which can be overwritten using `RUST_LOG`.
//! This will include all [`DEBUG`](tracing::Level::DEBUG) messages in debug mode,
//! and all [`INFO`](tracing::Level::INFO) level messages in release mode.
//!
//! If a `tracing` backend is already configured, this will not overwrite that.

use std::fs::File;
use std::time::UNIX_EPOCH;

use time::macros::format_description;
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[cfg(not(target_arch = "wasm32"))]
/// Initialise tracing for a non-web platform with the given `default_level`.
fn try_init_layered_tracing(default_level: LevelFilter) -> Result<(), SetGlobalDefaultError> {
    // Use EnvFilter to allow the user to override the log level without recompiling.
    let env_filter_builder = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .with_env_var("RUST_LOG");
    // We store the error until our env is set, *then* we display it
    let env_var_error = env_filter_builder.from_env().err();
    let env_filter = env_filter_builder.from_env_lossy();

    // This format is more concise than even the 'Compact' default:
    // - We print the time without the date (GUI apps usually run for very short periods).
    // - We print the time with millisecond instead of microsecond precision.
    // - We skip the target. In app code, the target is almost always visual noise. By
    //   default, it only gives you the module a log was defined in. This is rarely useful;
    //   the log message is much more helpful for finding a log's location.
    let timer = UtcTime::new(format_description!(
        // We append a `Z` here to indicate clearly that this is a UTC time
        "[hour repr:24]:[minute]:[second].[subsecond digits:3]Z"
    ));
    // If modifying, also update the module level docs
    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(timer.clone())
        .with_target(false)
        .with_filter(env_filter);

    // We skip the layer which stores to a file in `--release` mode for performance.
    let log_file_layer = if cfg!(debug_assertions) {
        let id = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let tmp_path = std::env::temp_dir().join(format!("masonry-{id:016}-dense.log"));
        // If modifying, also update the module level docs
        let log_file_layer = tracing_subscriber::fmt::layer()
            .with_timer(timer)
            .with_writer(File::create(&tmp_path).unwrap())
            // TODO - For some reason, `.with_ansi(false)` still leaves some italics in the output.
            .with_ansi(false);
        // Note that this layer does not use the provided filter, and instead logs all events.

        #[allow(clippy::print_stderr)]
        {
            // We print this message to stderr (rather than through `tracing`), because:
            // 1) Tracing hasn't been set up yet
            // 2) The tracing logs could have been configured to eat this message, and we think this is still important to have visible.
            // 3) This message is only sent in debug mode, so won't be exposed to users.
            eprintln!("---");
            eprintln!("Writing full logs to {}", tmp_path.display());
            eprintln!("---");
        }

        Some(log_file_layer)
    } else {
        None
    };

    #[cfg(target_os = "android")]
    let android_trace_layer = tracing_android_trace::AndroidTraceLayer::new();

    let registry = tracing_subscriber::registry()
        .with(console_layer)
        .with(log_file_layer);

    #[cfg(target_os = "android")]
    let registry = registry.with(android_trace_layer);

    // After the above line because of https://github.com/linebender/android_trace/pull/17
    #[cfg(feature = "tracy")]
    let registry = registry.with(tracing_tracy::TracyLayer::default());

    tracing::dispatcher::set_global_default(registry.into())?;

    if let Some(err) = env_var_error {
        tracing::error!(
            err = &err as &dyn std::error::Error,
            "Failed to parse RUST_LOG environment variable"
        );
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
/// Initialise tracing for the web with the given `max_level`.
fn try_init_wasm_tracing(max_level: LevelFilter) -> Result<(), SetGlobalDefaultError> {
    // Note - tracing-wasm might not work in headless Node.js. Probably doesn't matter anyway,
    // because this is a GUI framework, so wasm targets will virtually always be browsers.

    // Ignored if the panic hook is already set
    console_error_panic_hook::set_once();

    let config = tracing_wasm::WASMLayerConfigBuilder::new()
        .set_max_level(max_level)
        .build();

    tracing::subscriber::set_global_default(
        Registry::default().with(tracing_wasm::WASMLayer::new(config)),
    )
}

/// Initialise tracing for a unit test.
/// This ignores most messages to limit noise (but will still log all messages to a file).
pub(crate) fn try_init_test_tracing() -> Result<(), SetGlobalDefaultError> {
    // For unit tests we want to suppress most messages.
    let default_level = LevelFilter::WARN;
    #[cfg(not(target_arch = "wasm32"))]
    {
        try_init_layered_tracing(default_level)
    }

    #[cfg(target_arch = "wasm32")]
    {
        try_init_wasm_tracing(default_level)
    }
}

/// Initialise tracing for an end-user application.
pub(crate) fn try_init_tracing() -> Result<(), SetGlobalDefaultError> {
    // Default level is DEBUG in --dev, INFO in --release, unless a level is passed.
    // DEBUG should print a few logs per low-density event.
    // INFO should only print logs for noteworthy things.
    let default_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        try_init_layered_tracing(default_level)
    }

    #[cfg(target_arch = "wasm32")]
    {
        try_init_wasm_tracing(default_level)
    }
}
