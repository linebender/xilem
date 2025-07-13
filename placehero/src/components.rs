// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::entities::Status;
use xilem::view::{
    CrossAxisAlignment, FlexExt, FlexSequence, FlexSpacer, MainAxisAlignment, button, flex,
    flex_row, inline_prose, label, prose,
};
use xilem::{FontWeight, TextAlign};

use crate::{Avatars, Placehero, status_html_to_plaintext};

mod timeline;
pub(crate) use timeline::timeline;

mod thread;
pub(crate) use thread::thread;

/// Renders the key parts of a Status, in a shared way.
///
/// This is the shared functionality between a timeline and the list of views.
// TODO: Determine our UX for boosting/reblogging.
// In particular, do we want to have the same design as "normal" Mastodon, where the
// avatar for the booster is shown in the "child" avatar.
fn base_status(avatars: &mut Avatars, status: &Status) -> impl FlexSequence<Placehero> + use<> {
    // TODO: This really should be Arced or something.
    let status_clone: Status = status.clone();
    // TODO: In theory, it's possible to reblog a reblog; it's not clear what happens in this case.
    debug_assert!(status.reblog.is_none(), "`base_status` can't show reblogs.");
    // We return a child list.
    (
        // Account info/message time
        flex_row((
            avatars.avatar(&status.account.avatar_static),
            flex((
                inline_prose(status.account.display_name.as_str())
                    .weight(FontWeight::SEMI_BOLD)
                    .text_alignment(TextAlign::Start)
                    .text_size(20.)
                    .flex(CrossAxisAlignment::Start),
                inline_prose(status.account.username.as_str())
                    .weight(FontWeight::SEMI_LIGHT)
                    .text_alignment(TextAlign::Start)
                    .flex(CrossAxisAlignment::Start),
            ))
            .main_axis_alignment(MainAxisAlignment::Start)
            .gap(1.),
            FlexSpacer::Flex(1.0),
            inline_prose(status.created_at.format("%Y-%m-%d %H:%M:%S").to_string())
                .text_alignment(TextAlign::End),
        ))
        .must_fill_major_axis(true),
        prose(status_html_to_plaintext(status.content.as_str())),
        flex_row((
            label(format!("💬 {}", status.replies_count)).flex(1.0),
            label(format!("🔄 {}", status.reblogs_count)).flex(1.0),
            label(format!("⭐ {}", status.favourites_count)).flex(1.0),
            button("View Replies", move |state: &mut Placehero| {
                state
                    .context_sender
                    .as_ref()
                    .unwrap()
                    .send(status_clone.id.clone())
                    .unwrap();
                state.show_context = Some(status_clone.clone());
                state.context = None;
            }),
        ))
        // TODO: The "extra space" amount actually ends up being zero, so this doesn't do anything.
        .main_axis_alignment(MainAxisAlignment::SpaceEvenly),
    )
}
