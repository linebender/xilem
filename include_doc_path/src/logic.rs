// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
}

fn github_to_raw(repo: &str) -> Result<String, ()> {
    const GH_PREFIX: &str = "https://github.com/";
    if !repo.starts_with(GH_PREFIX) {
        return Err(());
    }

    // Handle some common variants such as https://github.com/<owner>/<repo>.git
    let mut rest = repo[GH_PREFIX.len()..].trim_end_matches('/').to_string();
    rest = rest.trim_end_matches(".git").to_string();

    // Parse URL as "https://github.com/<owner>/<repo>"
    let mut parts = rest.split('/');
    let owner = parts.next().unwrap_or("");
    let name = parts.next().unwrap_or("");
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return Err(());
    }

    Ok(format!("https://raw.githubusercontent.com/{owner}/{name}"))
}

fn compile_error(msg: &str, span: Span) -> TokenStream {
    let lit = LitStr::new(msg, span);
    quote!(compile_error!(#lit))
}

pub(crate) fn include_doc_path_impl(
    relative_path: String,
    span: Span,
    force_local: bool,
    force_url: bool,
) -> TokenStream {
    // PARSE ENV VARIABLES

    let Some(manifest_dir) = env("CARGO_MANIFEST_DIR") else {
        return compile_error("Could not read env variable 'CARGO_MANIFEST_DIR'", span);
    };
    let doc_file_path = Path::new(&manifest_dir).join(&relative_path);

    let Some(repo_url) = env("CARGO_PKG_REPOSITORY") else {
        return compile_error("Could not read env variable 'CARGO_PKG_REPOSITORY'", span);
    };
    let Ok(data_url) = github_to_raw(&repo_url) else {
        let error_message = format!(
            "URL stored in 'CARGO_PKG_REPOSITORY' doesn't match 'https://github.com/<owner>/<repo>' - Actual value is '{repo_url}'"
        );
        return compile_error(&error_message, span);
    };

    let tag_prefix = std::env::var("CRATE_TAG_PREFIX")
        .ok()
        .unwrap_or_else(|| "v".into());
    let Some(pkg_version) = env("CARGO_PKG_VERSION") else {
        return compile_error("Could not read env variable 'CARGO_PKG_VERSION'", span);
    };
    let Some(pkg_name) = env("CARGO_PKG_NAME") else {
        return compile_error("Could not read env variable 'CARGO_PKG_NAME'", span);
    };

    let doc_file_url = format!("{data_url}/{tag_prefix}{pkg_version}/{pkg_name}/{relative_path}");

    // FLAG PRE-FALLBACK LOGIC

    if force_local {
        let out = LitStr::new(doc_file_path.to_str().unwrap(), span);
        return quote!(#out);
    }

    if force_url {
        let out = LitStr::new(&doc_file_url, span);
        return quote!(#out);
    }

    // ACTUAL FALLBACK LOGIC

    // First, check if the local file exists
    if std::fs::metadata(&doc_file_path).is_ok() {
        let out = LitStr::new(doc_file_path.to_str().unwrap(), span);
        return quote!(#out);
    }

    // If not and CHECK_DOC_PATHS is set, error out.
    let check_doc_paths = env("CHECK_DOC_PATHS").is_some();
    if check_doc_paths {
        let doc_file_path = doc_file_path.to_string_lossy();
        let error_message = format!("File '{doc_file_path}' not found");
        return compile_error(&error_message, span);
    }

    // Else, return Github URL
    let out = LitStr::new(&doc_file_url, span);
    quote!(#out)
}
