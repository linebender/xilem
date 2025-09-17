// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The login flow for Placehero.

use std::collections::HashMap;

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
/// This supports graceful upgrades of the data (for any version which
/// has been on the main branch in the last month).
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

/// Information associated about a single server.
///
/// This will be stored in the `user_data` folder.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct MastodonServer {
    /// The name most recently associated with the server.
    cached_name: Option<ArcStr>,
    // TODO: Store a picture?
    /// The client id of the OAuth app.
    client_id: String,
    /// The client secret of the OAuth app.
    client_secret: String,
    // TODO: Should always return 0.
    client_secret_expires_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// The most recent version of the `LoginData` struct.
struct LoginData {
    /// Information about each server you've connected to.
    /// Key is the server's hostname, e.g. mastodon.social.
    ///
    /// TODO: Presumably we don't want to support non-https servers?
    // TODO: Is it a valid assumption that a server will only ever be "connected to" once?
    servers: HashMap<ArcStr, MastodonServer>,
}
