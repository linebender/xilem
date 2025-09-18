// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The login flow for Placehero.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use xilem::{WidgetView, masonry::core::ArcStr, view::label};

/// The "global" state for a version of Placehero redesigned for login.
#[derive(Default)]
pub(crate) struct PlaceheroWithLogin {
    login: Option<LoginData>,
}

pub(crate) fn app_logic(
    _state: &mut PlaceheroWithLogin,
) -> impl WidgetView<PlaceheroWithLogin> + use<> {
    label("Todo")
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
/// The name "app" is a pretty poor one from Mastodon, because it insteaad more closely refers to
/// a "login session".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "version")]
enum StoredLoginData {
    #[serde(rename = "0")]
    V0(LoginData),
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
    scopes: Vec<String>,
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
struct MastodonServer {
    /// The name most recently associated with the server, to give immediate feedback on loading.
    cached_name: Option<ArcStr>,
    // TODO: Store a picture?
    /// The currently associated apps with this server.
    /// For new logins, the most recent app should be preferred, if it has the right
    /// app name, scopes and website.
    ///
    /// The key is the "generation" of app associated with this server.
    apps: BTreeMap<u64, RemoteApp>,
    /// The first unused generation.
    app_generation: u64,
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
    kind: AccountKind,
    #[serde(skip)]
    oauth_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persisted_token: Option<String>,
    server_url: String,
    server_app_generation: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// The most recent version of the `LoginData` struct.
struct LoginData {
    /// Information about each server you've connected to.
    /// Key is the server's hostname, e.g. mastodon.social.
    ///
    /// Note on privacy: Supporting only single application each server is fine
    /// because Mastodon doesn't expose which `id` was used for an application.
    ///
    /// We're assuming that all Mastodon servers supported are trusted.
    // TODO: Presumably we don't want to support non-https servers?
    servers: HashMap<ArcStr, MastodonServer>,
    accounts: Vec<RemoteAccount>,
}
