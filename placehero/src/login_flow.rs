// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The login flow for Placehero.

use xilem::{WidgetView, view::label};

/// The "global" state for a version of Placehero redesigned for login
#[derive(Default)]
pub(crate) struct PlaceheroWithLogin {}

pub(crate) fn app_logic(
    _state: &mut PlaceheroWithLogin,
) -> impl WidgetView<PlaceheroWithLogin> + use<> {
    label("Todo")
}
