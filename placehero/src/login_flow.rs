// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The login flow for Placehero.

use xilem::{
    WidgetView,
    core::one_of::Either,
    view::{CrossAxisAlignment, FlexExt, flex_col, label, prose},
};

use crate::login_flow::login_model::LoginData;

mod login_model;

pub(crate) struct PlaceheroWithLogin {
    login: Option<LoginData>,
}

impl PlaceheroWithLogin {
    pub(crate) fn new() -> Self {
        Self {
            login: LoginData::load(),
        }
    }
}

pub(crate) fn app_logic(
    state: &mut PlaceheroWithLogin,
) -> impl WidgetView<PlaceheroWithLogin> + use<> {
    let Some(login) = &mut state.login else {
        return Either::A(flex_col((
            prose("Error: Placehero not ran using cargo run.")
                .text_size(25.)
                .text_alignment(xilem::TextAlign::Center)
                .flex(CrossAxisAlignment::Center),
            prose(
                "Placehero only supports being executed on the machine where it was compiled, and only using `cargo run`.\n\
                        This is because it is primarily an example for Xilem, but it also stores user data, including login credentials.\n\
                        As such, it needs to ensure that user data is always stored in a consistent place, \
                        and determining where this should be cross-platform is out-of-scope.\n\
                        You can find Placehero in the `placehero` folder in https://github.com/linebender/xilem.",
            ).text_alignment(xilem::TextAlign::Left)
                .flex(CrossAxisAlignment::Start),
        )));
    };

    // If auto-login, login to that account
    // If no accounts, go-to "new account" flow
    // If accounts, show accounts, and button for "new account" flow (at the top for consistency?)
    //
    // New account flow:
    // - Textbox at top for server address
    // - Also show list of previously used servers
    //
    // On server confirmation screen (show name, image?); allow selecting if
    // you want to connect to the account in a read-only manner.
    // Also select if you want to persist the login token (default true if read-only).
    //
    // If server has `multiple_accounts` set to true (either manually or due to logging
    // in to multiple accounts, then use force_login).
    // At this point, make sure we have a suitable app registered for this client, then authenticate the
    // user and register the token (with the right scopes).
    // Use an input box for the token, saving it with the account.
    // Don't forget e.g. a cancel button here.

    Either::B(label("Todo"))
}
