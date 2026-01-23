// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

// TODO - Remove syn dependency?

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
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

fn literal_string(content: &str, span: Span) -> TokenStream {
    let lit = LitStr::new(content, span);
    quote!(#lit)
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
    let check_doc_paths = env("CHECK_DOC_PATHS").is_some();

    // PARSE ENV VARIABLES

    let cargo_env_fallback = if check_doc_paths {
        compile_error(
            "Could not read one of the cargo env variables `CARGO_MANIFEST_DIR`, `CARGO_PKG_REPOSITORY`, `CARGO_PKG_VERSION`, or `CARGO_PKG_NAME`",
            span,
        )
    } else {
        literal_string(&relative_path, span)
    };

    let Some(manifest_dir) = env("CARGO_MANIFEST_DIR") else {
        return cargo_env_fallback;
    };
    let doc_file_path = Path::new(&manifest_dir).join(&relative_path);

    let Some(repo_url) = env("CARGO_PKG_REPOSITORY") else {
        return cargo_env_fallback;
    };
    let Ok(data_url) = github_to_raw(&repo_url) else {
        let error_message = format!(
            "URL stored in 'CARGO_PKG_REPOSITORY' doesn't match 'https://github.com/<owner>/<repo>' - Actual value is '{repo_url}'"
        );
        return compile_error(&error_message, span);
    };

    let tag_prefix = "v";

    let Some(pkg_version) = env("CARGO_PKG_VERSION") else {
        return cargo_env_fallback;
    };
    let Some(pkg_name) = env("CARGO_PKG_NAME") else {
        return cargo_env_fallback;
    };

    let doc_file_url = format!("{data_url}/{tag_prefix}{pkg_version}/{pkg_name}/{relative_path}");

    // FLAG PRE-FALLBACK LOGIC

    if force_local {
        return literal_string(doc_file_path.to_str().unwrap(), span);
    }

    if force_url {
        return literal_string(&doc_file_url, span);
    }

    // ACTUAL FALLBACK LOGIC

    // First, check if the local file exists
    if std::fs::metadata(&doc_file_path).is_ok() {
        return literal_string(doc_file_path.to_str().unwrap(), span);
    }

    // If not and CHECK_DOC_PATHS is set, error out.
    if check_doc_paths {
        let doc_file_path = doc_file_path.to_string_lossy();
        let error_message = format!("File '{doc_file_path}' not found");
        return compile_error(&error_message, span);
    }

    // Else, return GitHub URL
    literal_string(&doc_file_url, span)
}
