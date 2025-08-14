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
    /// HACK: The null navigation, because Xilem's handling of optional/None actions is not good.
    ///
    /// We're considering something like Xilem Web's `OptionalAction` (plus
    /// [#xilem > View Generic Action = !](https://xi.zulipchat.com/#narrow/channel/354396-xilem/topic/View.20Generic.20Action.20.3D.20!/with/534016260))
    None,
}
