// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::Megalodon;
use megalodon::entities::{Account, Status};
use megalodon::megalodon::GetAccountStatusesInputOptions;
use xilem::WidgetView;
use xilem::core::fork;
use xilem::palette::css;
use xilem::style::Style;
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::view::{flex, prose, sized_box, virtual_scroll, worker_raw};

use super::base_status;
use crate::actions::Navigation;

const BUFFER: usize = 3;

struct TimelineContinuationRequest {
    /// The "max id" to send with the request.
    max_id: Option<String>,
}

/// The state needed to load and display a single timeline (i.e. a sequence of unrelated posts)
pub(crate) struct Timeline {
    /// The currently loaded statuses in the timeline.
    statuses: Vec<Status>,
    /// The virtual scroll index which `statuses` starts at (for when we
    /// support loading forward in time)
    start_index: i64,
    /// Whether we're currently expecting a response.
    pending_id: bool,
    /// The sender for newly requested items.
    requests: Option<UnboundedSender<TimelineContinuationRequest>>,
    // We can't cache here due to Avatars not being cache busting ready.
    // /// Cached views for each status, as their input won't change.
    // cached_statuses: HashMap<i64, Arc<AnyWidgetView<Timeline, Navigation>>>,
    // TODO: Generalise to e.g. the explore page.
    user_id: String,
}

impl Timeline {
    pub(crate) fn new_for_account(account: Account) -> Self {
        Self {
            statuses: Vec::default(),
            start_index: 0,
            pending_id: false,
            requests: None,
            user_id: account.id,
        }
    }
}

/// A single timeline, i.e. the posts sent by a single user.
///
/// These statuses are currently not rendered with a reply indicator, etc.
/// and own their own boxes
pub(crate) fn timeline(
    timeline: &mut Timeline,
    mastodon: crate::Mastodon,
) -> impl WidgetView<Timeline, Navigation> + use<> {
    let user = timeline.user_id.clone();
    fork(
        virtual_scroll(
            timeline.start_index
                ..(timeline.start_index + i64::try_from(timeline.statuses.len()).unwrap()),
            |timeline: &mut Timeline, idx| {
                let local_idx = usize::try_from(idx - timeline.start_index).unwrap();
                if local_idx + BUFFER >= timeline.statuses.len() && !timeline.pending_id {
                    timeline.pending_id = true;
                    timeline
                        .requests
                        .as_ref()
                        .unwrap()
                        .send(TimelineContinuationRequest {
                            max_id: Some(timeline.statuses.last().unwrap().id.clone()),
                        })
                        .unwrap();
                }
                timeline_status(&timeline.statuses[local_idx])
                // match timeline.cached_statuses.entry(idx) {
                //     hash_map::Entry::Occupied(occupied_entry) => occupied_entry.get().clone(),
                //     hash_map::Entry::Vacant(vacant_entry) => vacant_entry
                //         .insert(Arc::new(timeline_status(&timeline.statuses[local_idx])))
                //         .clone(),
                // }
            },
        ),
        worker_raw(
            move |proxy, mut recv: UnboundedReceiver<TimelineContinuationRequest>| {
                let user = user.clone();
                let mastodon = mastodon.clone();
                async move {
                    while let Some(next) = recv.recv().await {
                        let result = mastodon
                            .get_account_statuses(
                                user.clone(),
                                Some(&GetAccountStatusesInputOptions {
                                    max_id: next.max_id,
                                    exclude_reblogs: Some(false),
                                    exclude_replies: Some(true),
                                    ..Default::default()
                                }),
                            )
                            .await;
                        drop(proxy.message(result));
                    }
                }
            },
            |timeline: &mut Timeline, sender| {
                timeline.requests = Some(sender);
                if timeline.statuses.is_empty() {
                    // TODO: This is a nasty hack
                    timeline
                        .requests
                        .as_ref()
                        .unwrap()
                        .send(TimelineContinuationRequest { max_id: None })
                        .unwrap();
                }
            },
            |timeline: &mut Timeline, resp| {
                match resp {
                    Ok(mut instance) => {
                        // If we're at the oldest post, there's no use making the same request again
                        if !instance.json.is_empty() {
                            timeline.pending_id = false;
                        }
                        timeline.statuses.append(&mut instance.json);
                    }
                    Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                        todo!()
                    }
                    Err(megalodon::error::Error::RequestError(e)) if e.is_status() => {
                        todo!()
                    }
                    Err(e) => {
                        todo!("handle {e}")
                    }
                }
                Navigation::None
            },
        ),
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
pub(crate) fn timeline_status(status: &Status) -> impl WidgetView<Timeline, Navigation> + use<> {
    let (info_line, primary_status) = if let Some(reblog) = status.reblog.as_ref() {
        (
            Some(prose(format!("🔄 {} boosted", status.account.display_name))),
            &**reblog,
        )
    } else {
        (None, status)
    };
    sized_box(flex((info_line, base_status(primary_status))))
        .border(css::WHITE, 2.0)
        .padding(10.0)
        .corner_radius(5.)
}
