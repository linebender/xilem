// Copyright 2021 the Xilem Authors and Armin Ronacher
// SPDX-License-Identifier: Apache-2.0

// This is based on: https://github.com/mitsuhiko/insta/blob/660f2b00e3092de50d4f7a59f28336d8a9da50b7/src/env.rs

// TODO - clean this all up - See https://github.com/linebender/xilem/issues/363

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use serde::Deserialize;

static WORKSPACES: Lazy<Mutex<BTreeMap<String, Arc<PathBuf>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Return the cargo workspace for a manifest
pub(crate) fn get_cargo_workspace(manifest_dir: &str) -> Arc<PathBuf> {
    // we really do not care about poisoning here.
    let mut workspaces = WORKSPACES.lock().unwrap_or_else(|x| x.into_inner());
    if let Some(rv) = workspaces.get(manifest_dir) {
        rv.clone()
    } else {
        #[derive(Deserialize)]
        struct Manifest {
            workspace_root: PathBuf,
        }
        let output = std::process::Command::new(
            env::var("CARGO")
                .ok()
                .unwrap_or_else(|| "cargo".to_string()),
        )
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--no-deps")
        .current_dir(manifest_dir)
        .output()
        .unwrap();
        let manifest: Manifest = serde_json::from_slice(&output.stdout).unwrap();
        let path = Arc::new(manifest.workspace_root);
        workspaces.insert(manifest_dir.to_string(), path.clone());
        path
    }
}
