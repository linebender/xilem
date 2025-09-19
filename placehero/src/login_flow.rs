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
    Either::B(label("Todo"))
}
