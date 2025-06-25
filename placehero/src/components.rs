// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem::FontWeight;
use xilem::WidgetView;
use xilem::palette::css;
use xilem::style::Padding;
use xilem::style::Style;
use xilem::view::FlexSequence;
use xilem::view::flex_row;
use xilem::view::portal;
use xilem::view::{
    CrossAxisAlignment, FlexExt, FlexSpacer, MainAxisAlignment, flex, inline_prose, label, prose,
    sized_box,
};

use super::Placehero;
use crate::avatars::Avatars;
use crate::html_content::status_html_to_plaintext;

use megalodon::entities::Status;

/// The component for a single status in a [`timeline`].
///
/// These statuses are currently not currently rendered with a reply indicator, media, etc.
/// This is planned. Reblogged statuses are also not currently handled correctly.
///
/// They are rendered with a surrounding padded box.
// TODO: Work out how much of this component can be reused in a reply timeline.
// I think you want the same thing, but without the box, and without any "this is a reply" indicator.
// It also wouldn't need to handle reblogs (the API doesn't provide any way to make a reply status which is a reblog).
// N.b. API wise, there's no reason that you can't reply to a "reblog" status. TODO: Confirm this
pub(crate) fn timeline_status(
    avatars: &mut Avatars,
    status: &Status,
) -> impl WidgetView<Placehero> + use<> {
    let (info_line, primary_status) = if let Some(reblog) = status.reblog.as_ref() {
        (
            Some(prose(format!("🔄 {} boosted", status.account.display_name))),
            &**reblog,
        )
    } else {
        (None, status)
    };
    sized_box(flex((info_line, base_status(avatars, primary_status))))
        .border(css::WHITE, 2.0)
        .padding(10.0)
        .corner_radius(5.)
}

/// Renders the key parts of a Status, in a shared way.
///
/// This is the shared funcitionality between a timeline and the list of views.
// TODO: Determine our UX for boosting/reblogging.
// In particular, do we want to have the same design as "normal" Mastodon, where the
// avatar for the booster is shown in the "child" avatar.
fn base_status(avatars: &mut Avatars, status: &Status) -> impl FlexSequence<Placehero> + use<> {
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
                    .alignment(xilem::TextAlignment::Start)
                    .text_size(20.)
                    .flex(CrossAxisAlignment::Start),
                inline_prose(status.account.username.as_str())
                    .weight(FontWeight::SEMI_LIGHT)
                    .alignment(xilem::TextAlignment::Start)
                    .flex(CrossAxisAlignment::Start),
            ))
            .main_axis_alignment(MainAxisAlignment::Start)
            .gap(1.),
            FlexSpacer::Flex(1.0),
            inline_prose(status.created_at.format("%Y-%m-%d %H:%M:%S").to_string())
                .alignment(xilem::TextAlignment::End),
        ))
        .must_fill_major_axis(true),
        prose(status_html_to_plaintext(status.content.as_str())),
        flex_row((
            label(format!("💬 {}", status.replies_count)).flex(1.0),
            label(format!("🔄 {}", status.reblogs_count)).flex(1.0),
            label(format!("⭐ {}", status.favourites_count)).flex(1.0),
        ))
        // TODO: The "extra space" amount actually ends up being zero, so this doesn't do anything.
        .main_axis_alignment(MainAxisAlignment::SpaceEvenly),
    )
}

/// A [`timeline`]; statuses are rendered individually.
///
/// These statuses are currently not rendered with a reply indicator, etc.
/// and own their own boxes
pub(crate) fn timeline(
    statuses: &mut [Status],
    avatars: &mut Avatars,
) -> impl WidgetView<Placehero> + use<> {
    portal(
        flex(
            statuses
                .iter()
                .map(|status| timeline_status(avatars, status))
                .collect::<Vec<_>>(),
        )
        .padding(Padding {
            // Leave room for scrollbar
            right: 20.,
            ..Padding::all(5.0)
        }),
    )
}
