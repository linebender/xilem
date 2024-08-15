// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fs::File;
use std::time::UNIX_EPOCH;

use time::macros::format_description;
use tracing::subscriber::SetGlobalDefaultError;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[cfg(target_arch = "wasm32")]
pub(crate) fn try_init_wasm_tracing() -> Result<(), SetGlobalDefaultError> {
    // Note - tracing-wasm might not work in headless Node.js. Probably doesn't matter anyway,
    // because this is a GUI framework, so wasm targets will virtually always be browsers.

    // Ignored if the panic hook is already set
    console_error_panic_hook::set_once();

    let max_level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    let config = tracing_wasm::WASMLayerConfigBuilder::new()
        .set_max_level(max_level)
        .build();

    tracing::subscriber::set_global_default(
        Registry::default().with(tracing_wasm::WASMLayer::new(config)),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn try_init_layered_tracing(
    default_level: Option<LevelFilter>,
) -> Result<(), SetGlobalDefaultError> {
    // Default level is DEBUG in --dev, INFO in --release, unless a level is passed.
    // DEBUG should print a few logs per low-density event.
    // INFO should only print logs for noteworthy things.
    let default_level = if let Some(level) = default_level {
        level
    } else if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
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
        // TODO - For some reason, `.with_ansi(false)` still leaves some italics in the output.
        let log_file_layer = tracing_subscriber::fmt::layer()
            .with_timer(timer)
            .with_writer(File::create(tmp_path.clone()).unwrap())
            .with_ansi(false);
        println!("---");
        println!("Writing full logs to {}", tmp_path.to_string_lossy());
        println!("---");
        Some(log_file_layer)
    } else {
        None
    };

    let registry = tracing_subscriber::registry()
        .with(console_layer)
        .with(log_file_layer);

    tracing::dispatcher::set_global_default(registry.into())?;

    if let Some(err) = env_var_error {
        tracing::error!("Failed to parse RUST_LOG environment variable: {err}");
    }

    Ok(())
}

pub(crate) fn try_init_test_tracing() -> Result<(), SetGlobalDefaultError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // For unit tests we want to suppress most messages.
        try_init_layered_tracing(Some(LevelFilter::WARN))
    }

    #[cfg(target_arch = "wasm32")]
    {
        try_init_wasm_tracing()
    }
}

pub(crate) fn try_init_tracing() -> Result<(), SetGlobalDefaultError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        try_init_layered_tracing(None)
    }

    #[cfg(target_arch = "wasm32")]
    {
        try_init_wasm_tracing()
    }
}
