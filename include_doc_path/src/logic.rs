// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

// TODO - Remove syn dependency?

fn get_env_var(key: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn github_to_raw(repo: &str) -> Result<String, ()> {
    const GH_PREFIX: &str = "https://github.com/";

    let Some(rest) = repo.strip_prefix(GH_PREFIX) else {
        return Err(());
    };

    // Handle some common variants such as https://github.com/<owner>/<repo>.git
    let rest = rest.trim_end_matches('/');
    let rest = rest.trim_end_matches(".git").to_string();

    // Parse "<owner>/<repo>" part of the URL
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

    let manifest_dir = get_env_var("CARGO_MANIFEST_DIR");
    let doc_file_path = Path::new(&manifest_dir).join(&relative_path);

    let repo_url = get_env_var("CARGO_PKG_REPOSITORY");
    let Ok(data_url) = github_to_raw(&repo_url) else {
        let error_message = format!(
            "URL stored in 'CARGO_PKG_REPOSITORY' doesn't match 'https://github.com/<owner>/<repo>' - Actual value is '{repo_url}'"
        );
        return compile_error(&error_message, span);
    };

    let tag_prefix = "v";

    let pkg_version = get_env_var("CARGO_PKG_VERSION");
    let pkg_name = get_env_var("CARGO_PKG_NAME");

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
    let check_doc_paths = std::env::var("CHECK_DOC_PATHS") == Ok("true".to_string());
    if check_doc_paths {
        let doc_file_path = doc_file_path.to_string_lossy();
        let error_message = if doc_file_path.contains("<unknown>") {
            format!(
                "File '{doc_file_path}' not found. This is likely because Cargo environment variables are unset."
            )
        } else {
            format!("File '{doc_file_path}' not found.")
        };
        return compile_error(&error_message, span);
    }

    // Else, return GitHub URL
    let out = LitStr::new(&doc_file_url, span);
    quote!(#out)
}
