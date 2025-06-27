// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::entities::{Context, Status};
use xilem::WidgetView;
use xilem::palette::css;
use xilem::style::{Padding, Style};
use xilem::view::{CrossAxisAlignment, FlexExt, flex, flex_row, label, portal, sized_box};

use crate::components::base_status;
use crate::{Avatars, Placehero};

/// Display a status in the context of its thread.
///
/// Notes:
/// 1) We don't try and do anything "fancy", i.e. we just display all the items as one thing.
pub(crate) fn thread(
    avatars: &mut Avatars,
    root_status: &Status,
    // TODO: Maybe the context should be optional (for async loading)
    // The hard part there would be locking the scroll properly (i.e. once the thread loads)
    thread: &Context,
    // TODO: Think about allowing composing a reply.
) -> impl WidgetView<Placehero> + use<> {
    let mut ancestor_views = Vec::new();
    let mut previous_parent = None;
    for ancestor in &thread.ancestors {
        if previous_parent != ancestor.in_reply_to_id.as_deref() {
            if previous_parent.is_none() {
                tracing::warn!("Couldn't load all ancestors, presumably due to context limits?");
            } else {
                // TODO: This should maybe be `debug_panic`, but that's not exposed currently.
                panic!("For correct ordering, we currently assume that the Mastodon API gives");
            }
        }
        previous_parent = ancestor.in_reply_to_id.as_deref();
        ancestor_views.push(thread_ancestor(avatars, ancestor));
    }
    // TODO: Determine depth; maybe turn into a "real" tree.
    let mut descendant_views = Vec::new();
    for descendant in &thread.descendants {
        descendant_views.push(thread_ancestor(avatars, descendant));
    }

    portal(
        flex((
            ancestor_views,
            base_status(avatars, root_status),
            label("Replies:"),
            descendant_views,
        ))
        .padding(Padding {
            // Leave room for scrollbar
            right: 20.,
            ..Padding::all(5.0)
        }),
    )
}

fn thread_ancestor(avatars: &mut Avatars, status: &Status) -> impl WidgetView<Placehero> + use<> {
    sized_box(
        flex_row((
            // An awful left-side border.
            sized_box(flex(()))
                .width(3.)
                .height(50.)
                .background_color(css::WHITE)
                .flex(CrossAxisAlignment::Start),
            flex(base_status(avatars, status)).flex(1.0),
        ))
        .must_fill_major_axis(true),
    )
}
