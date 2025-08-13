// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::entities::Status;

/// Ways that the app can navigate within itself.
#[expect(clippy::large_enum_variant, reason = "Who cares?")]
pub(crate) enum Navigation {
    /// Load the context (i.e. replies and ancestors) of a given
    /// (non-repost) status.
    LoadContext(Status),
    /// Return to the main timeline.
    // TODO: Maintain scroll state in the timeline.
    // TODO: More of a back stack.
    Home,
}
