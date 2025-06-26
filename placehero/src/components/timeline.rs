use megalodon::entities::Status;
use xilem::WidgetView;
use xilem::palette::css;
use xilem::style::{Padding, Style};
use xilem::view::{flex, portal, prose, sized_box};

use super::base_status;
use crate::{Avatars, Placehero};

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
