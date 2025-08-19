// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use megalodon::Megalodon;
use megalodon::entities::{Account, Status};
use megalodon::megalodon::GetAccountStatusesInputOptions;
use xilem::core::fork;
use xilem::core::one_of::{OneOf, OneOf3};
use xilem::masonry::core::ArcStr;
use xilem::masonry::properties::types::AsUnit;
use xilem::palette::css;
use xilem::style::Style;
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::view::{flex, prose, sized_box, spinner, virtual_scroll, worker_raw};
use xilem::{TextAlign, WidgetView};

use super::base_status;
use crate::Mastodon;
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
    /// Whether we've reached the end of this timeline
    at_end: bool,
    /// The sender for newly requested items.
    requests: Option<UnboundedSender<TimelineContinuationRequest>>,
    // We can't cache here due to Avatars not being cache busting ready.
    // /// Cached views for each status, as their input won't change.
    // cached_statuses: HashMap<i64, Arc<AnyWidgetView<Timeline, Navigation>>>,
    // TODO: Generalise to e.g. the explore page.
    user_id: ArcStr,
}

impl Timeline {
    pub(crate) fn new_for_account(account: Account) -> Self {
        Self {
            statuses: Vec::default(),
            start_index: 0,
            pending_id: false,
            requests: None,
            user_id: account.id.into(),
            at_end: false,
        }
    }

    pub(crate) fn view(&mut self, mastodon: Mastodon) -> impl WidgetView<Self, Navigation> + use<> {
        // We clone the relevant user id for use in `worker_raw`
        // (We plan for the function which makes the future to have access to the app state, but that hasn't happened yet)
        let user = self.user_id.clone();
        fork(
            virtual_scroll(
                // Show the statuses which have been loaded
                // We currently never unload previous statuses (note that the widgets but not the remainder are)
                // +1 to allow room for the spinner.
                // Note that we also launch the very first initial request from the spinner
                self.start_index
                    ..(self.start_index + i64::try_from(self.statuses.len()).unwrap() + 1),
                |timeline: &mut Self, idx| {
                    let local_idx = usize::try_from(idx - timeline.start_index).unwrap();
                    // If we're "close" to the last downloaded item.
                    if local_idx + BUFFER >= timeline.statuses.len()
                        // And don't already have a request in flight
                        && !timeline.pending_id
                        && !timeline.at_end
                    {
                        // Kick off the next request
                        timeline.pending_id = true;
                        timeline
                            .requests
                            .as_ref()
                            .unwrap()
                            .send(TimelineContinuationRequest {
                                // The max id is the newest status which will be excluded; i.e. we request
                                // all the statuses from before the last loaded one
                                // (If we haven't loaded any yet, we just load the first n)
                                max_id: timeline.statuses.last().map(|it| it.id.clone()),
                            })
                            .unwrap();
                    }
                    if local_idx == timeline.statuses.len() {
                        if timeline.at_end {
                            OneOf3::A(prose("End of timeline.").text_alignment(TextAlign::Center))
                        } else {
                            OneOf::B(sized_box(spinner()).width(50.px()).height(50.px()))
                        }
                    } else {
                        // We would like to cache the status, but this is currently not supported due to `Avatars`
                        // (and everything environment-system) not being properly rebuild aware.
                        // This is planned, but the infra isn't in place yet.
                        // match timeline.cached_statuses.entry(idx) {
                        //     hash_map::Entry::Occupied(occupied_entry) => occupied_entry.get().clone(),
                        //     hash_map::Entry::Vacant(vacant_entry) => vacant_entry
                        //         .insert(Arc::new(timeline_status(&timeline.statuses[local_idx])))
                        //         .clone(),
                        // }
                        OneOf::C(timeline_status(&timeline.statuses[local_idx]))
                    }
                },
            ),
            worker_raw(
                move |proxy, mut recv: UnboundedReceiver<TimelineContinuationRequest>| {
                    let user = user.clone();
                    let mastodon = mastodon.clone();
                    async move {
                        // For every request we receive (there should only ever be one
                        // at a time, but worker only supports unbounded queues at the moment)
                        // we load the requested statuses
                        while let Some(next) = recv.recv().await {
                            let result = mastodon
                                .get_account_statuses(
                                    (*user).into(),
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
                |timeline: &mut Self, sender| {
                    // `worker` creates a channel pair for us; we need to keep our own track
                    // of the sender
                    timeline.requests = Some(sender);
                },
                |timeline: &mut Self, resp| {
                    match resp {
                        Ok(mut instance) => {
                            // If we get an empty response, that means we're at the end of
                            // the timeline (...probably, this is undocumented)
                            if instance.json.is_empty() {
                                timeline.at_end = true;
                            }
                            timeline.pending_id = false;
                            timeline.statuses.append(&mut instance.json);
                        }
                        Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
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
}

/// The component for a single status in a [`Timeline`].
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
