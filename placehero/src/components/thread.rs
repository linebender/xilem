// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::entities::{Context, Status};
use xilem::WidgetView;
use xilem::core::Edit;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::util::debug_panic;
use xilem::palette::css;
use xilem::style::{Padding, Style};
use xilem::view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, label, portal, sized_box};

use crate::Placehero;
use crate::actions::Navigation;
use crate::components::base_status;

/// Display a status in the context of its thread.
///
/// Notes:
/// 1) We don't try and do anything "fancy", i.e. we just display all the items in a column.
///    That is, we don't have an "increasing depth" threading UI.
pub(crate) fn thread(
    root_status: &Status,
    // TODO: Maybe the context should be optional (for async loading)
    // The hard part there would be locking the scroll properly (i.e. once the thread loads)
    thread: &Context,
    // TODO: Think about allowing composing a reply.
) -> impl WidgetView<Edit<Placehero>, Navigation> + use<> {
    let mut ancestor_views = Vec::new();
    let mut previous_parent = None;
    for ancestor in &thread.ancestors {
        if previous_parent != ancestor.in_reply_to_id.as_deref() {
            if previous_parent.is_none() {
                tracing::warn!(
                    "Couldn't load all ancestors, presumably due to unauthenticated context length limits?"
                );
            } else {
                debug_panic!(
                    "For simplicity, we assume that ancestors are returned in reading order \
                    from the Mastodon API, but this was violated.\n\
                    Status {} had parent {:?}, but expected {previous_parent:?}",
                    ancestor.id,
                    ancestor.in_reply_to_id
                );
            }
        }
        previous_parent = Some(&ancestor.id);
        ancestor_views.push(thread_ancestor(ancestor));
    }
    // TODO: Determine depth; maybe turn into a "real" tree.
    let mut descendant_views = Vec::new();
    for descendant in &thread.descendants {
        descendant_views.push(thread_ancestor(descendant));
    }

    portal(
        flex_col((
            ancestor_views,
            base_status(root_status),
            label("Replies:").flex(CrossAxisAlignment::Start),
            descendant_views,
        ))
        .padding(Padding {
            // Leave room for scrollbar
            right: 20.,
            ..Padding::all(5.0)
        }),
    )
}

/// The component for a single post in a thread.
///
/// These are rendered without a containing box, and with an adjoining "reply indicator"
/// (which is currently known to be terrible!).
fn thread_ancestor(status: &Status) -> impl WidgetView<Edit<Placehero>, Navigation> + use<> {
    sized_box(
        flex_row((
            // An awful left-side border.
            sized_box(flex_col(()))
                .width(3.px())
                .height(50.px())
                .background_color(css::WHITE)
                .flex(CrossAxisAlignment::Start),
            flex_col(base_status(status)).flex(1.0),
        ))
        .must_fill_major_axis(true),
    )
}
