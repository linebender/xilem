// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A data model and flow for Mastodon-style oauth login.
//!
//! This is entirely separate from the UI layers (except for choices around string types).
//!
//! This currently requires that the program was executed using `cargo run`, and stores all data in `placehero/user_data`.
//! We make these choices because:
//! - We're support storing sensitive data (i.e. tokens), and it's helpful if we always keep this
//!   consistently in a single place.
//! - We are an example, and so don't want to pollute the user's home directory, etc.
//!
//! If you're using this code to implement your own Mastodon (or similar) client implementation,
//! you should choose a data location relevant to your install method.

#![expect(dead_code, reason = "In progress")]

// TODO: How can we test this? Would we want to make a local Mastodon server?

use std::{
    collections::{BTreeMap, HashSet},
    io,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use xilem::masonry::core::ArcStr;

#[derive(Serialize, Deserialize, Clone, Debug)]
/// The most recent version of the `LoginData` struct.
pub(super) struct LoginData {
    // TODO: Is this idiomatic? Would we more ideally have a separate "stored" model?
    #[serde(skip)] // Hydrated as part of load procedure
    /// The file path where this data will be stored.
    file_path: PathBuf,
    /// Information about each server you've connected to.
    /// Key is the server's hostname, e.g. "mastodon.social".
    // TODO: Presumably we don't want to support non-https servers?
    // We want to display the servers in alphabetical order.
    servers: BTreeMap<ArcStr, MastodonServer>,
    accounts: Vec<RemoteAccount>,
}

impl LoginData {
    /// Load the login data from disk, updating it to the latest format if possible.
    ///
    /// See the module level data for information about where the login data is stored.
    ///
    /// This function returns `None` if we're not executed by using `cargo run`.
    // TODO: Would it be worth loading this async? Probably not.
    pub(super) fn load() -> Option<Self> {
        let file_path = login_file_path()?;
        let data = match std::fs::read_to_string(&file_path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Some(Self {
                    file_path,
                    servers: BTreeMap::new(),
                    accounts: Vec::new(),
                });
            }
            Err(e) => {
                // TODO: Handle through GUI?
                panic!("TODO: Handle {e:?} whilst loading user data.")
            }
        };
        let stored_data = match serde_json::from_str::<StoredLoginData>(&data) {
            Ok(stored_data) => stored_data,
            Err(e) => {
                // TODO: Handle through GUI?
                // We intentionally *don't* backup this data.
                panic!("Stored login data is not valid JSON or not in an expected format: {e:?}");
            }
        };
        // We currently only have V0, so just unwrap it.
        let StoredLoginData::V0(mut data) = stored_data;
        data.file_path = file_path;
        for account in &mut data.accounts {
            // This should be completely impossible, but given how scary it is, better to be safe than sorry.
            assert!(
                account.oauth_token.is_none(),
                "OAuth token was unexpectedly saved in login data.\n\
                This indicates a critical bug in Placehero, please report it at https://github.com/linebender/xilem.\n\
                You might want to invalidate that token (by logging into the account\
                (on https://{}) and removing the Placehero app's authorisation.",
                account.server_url
            );
            account.oauth_token = account.persisted_token.clone();
        }
        Some(data)
    }

    pub(super) fn servers(&self) -> &BTreeMap<ArcStr, MastodonServer> {
        &self.servers
    }
}

/// Get the path to the login data file.
///
/// That file might not actually exist yet.
/// This function returns `None` if we're not executed by using `cargo run`.
fn login_file_path() -> Option<PathBuf> {
    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        // Note that we also give GUI guidance to the user.
        tracing::warn!("Placehero wasn't executed under `cargo run`.");
        return None;
    };
    let mut path = PathBuf::from(manifest_dir);
    if !path.ends_with("placehero") {
        tracing::debug!(
            manifest_path = ?path,
            "Placehero code might have been copied to different project."
        );
        assert!(
            path.ends_with("placehero"),
            "Placehero's login data storage is only designed for its use as an example.\n\
            If you wish to adapt this code for another project, you should ensure that you evaluate where you store the data\
            (e.g. in a location idiomatic to your target platform, with sufficient token security in place).",
        );
    }
    path.push("user_data/login.json");
    Some(path)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RemoteApp {
    /// The client id of the OAuth app.
    client_id: String,
    /// The client secret of the OAuth app.
    client_secret: String,
    /// The "scopes" we created this application with.
    /// Used to determine whether we need to recreate the application, because our
    /// set of required scopes has changed.
    ///
    /// Ideally, we'd recover these dynamically, but that isn't currently feasible as Megalodon
    /// doesn't expose this, and also doesn't expose the right fields in `verify_credentials`.
    scopes: HashSet<String>,
    /// The redirection URIs we created this application with. Same caveat as `scopes`.
    redirect_uris: Vec<String>,
    /// The "name" we created this app with.
    ///
    /// Stored in case we ever change the name of the Placehero app, we want to recreate the application.
    /// N.B. we want to do this dynamically at each login; that is, we don't need to do this for accounts
    /// which are logged-in but dormant, at least not immediately. Same caveat as `scopes`.
    ///
    /// This is amongst the reasons that [`MastodonServer::apps`] has generations.
    app_name: String,
    /// The "website url" we created this app with. Same caveat as `scopes`.
    app_website: String,
    // These fields can be added backwards-compatibly using:
    // These would be used if we ever change the URL and/or name of the application.
    // (This should be opt-in because it requires the user to re-authenticate)
    // #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    // allow_outdated_name: bool,
    // #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    // allow_outdated_url: bool,
}
/// Information associated about a single server which the user has clients on.
///
/// This will be stored in the `user_data` folder.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct MastodonServer {
    /// The name most recently associated with the server, to give immediate feedback on loading.
    pub(crate) cached_name: ArcStr,
    // TODO: Store a picture?
    /// The currently associated apps with this server.
    /// For new logins, the most recent app should be preferred, if it has the right
    /// app name, scopes and website.
    ///
    /// The key is the "generation" of app associated with this server.
    ///
    /// Note on privacy: Sharing application within each server is fine
    /// because Mastodon doesn't expose which `id` was used for an application.
    ///
    /// We're assuming that all Mastodon servers supported are trusted (and so it's not a
    /// privacy violation for them to know that two accounts are linked).
    ///
    /// `BTreeMap` because we regularly need to get the "last" value; in most
    /// cases we expect there to be fewer than 4 items.
    apps: BTreeMap<u64, RemoteApp>,
    /// The first unused generation.
    app_generation: u64,
    /// For login, whether the user has multiple accounts on this server
    /// (and therefore will want to ).
    multiple_accounts: bool,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
enum AccountKind {
    ReadOnly,
    ReadWrite,
}

/// The scopes added to new "apps" by Placehero.
///
/// This is the maximum required scopes used; a subset of these are used if you make an
/// [`AccountKind::ReadOnly`].
///
/// Currently only "read"-style scopes, as no features which use writing have been added.
const ALL_REQUIRED_SCOPES: [&str; 2] = ["profile", "read"];

impl AccountKind {
    fn required_scopes(self) -> &'static [&'static str] {
        match self {
            // TODO: Split into finer grained scopes.
            Self::ReadOnly => &["profile", "read"],
            // TODO: Start to implement writing features.
            Self::ReadWrite => &ALL_REQUIRED_SCOPES,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RemoteAccount {
    /// The username most recently associated with the account, to give immediate feedback on loading.
    cached_name: String,
    kind: AccountKind,
    /// The active token as part of this run of Placehero.
    #[serde(skip)] // Hydrated to equal persisted_token as part of load procedure
    oauth_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persisted_token: Option<String>,
    server_url: String,
    server_app_generation: u64,
}

/// The data stored in the file `user_data/login.json`.
///
/// This data design ensures that we support graceful upgrades of the data (but note that for simplicity
/// we will likely not implement this).
///
/// Our login strategy is broadly as follows:
/// - Once per server (which you're logging in from), we create an app, will all of the potentially used scopes.
/// - If the previously created app doesn't have the right scopes, we make a new one (after revoking the login token to the old app).
/// - We then log the user in to this app, with the scopes they request.
///    - We by default check "auto-login" for read-only scopes
///    - We support auto-login for read-write scopes
///
/// The name "app" is a pretty poor one from Mastodon, because it instead more closely refers to
/// a "login session".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "version")]
enum StoredLoginData {
    #[serde(rename = "0")]
    V0(LoginData),
}
