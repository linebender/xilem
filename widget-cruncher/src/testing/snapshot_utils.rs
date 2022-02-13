// Shamelessly stolen from Insta - probably fine because Insta has the Apache license
// TODO - ask permission
// TODO - clean this up

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs};

use once_cell::sync::Lazy;
use serde::Deserialize;

#[macro_export]
macro_rules! assert_render_snapshot {
    ($test_harness:expr, $name:expr) => {
        $test_harness.check_render_snapshot(
            env!("CARGO_MANIFEST_DIR"),
            file!(),
            module_path!(),
            $name,
        )
    };
}

static WORKSPACES: Lazy<Mutex<BTreeMap<String, Arc<PathBuf>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Returns the cargo workspace for a manifest
pub fn get_cargo_workspace(manifest_dir: &str) -> Arc<PathBuf> {
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
