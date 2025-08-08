// Copyright 2021 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// TODO - See https://github.com/linebender/xilem/issues/336

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    reason = "Deferred: Tests need to be refactored"
)]

mod layout;
mod status_change;
mod transforms;
mod widget_tree;
