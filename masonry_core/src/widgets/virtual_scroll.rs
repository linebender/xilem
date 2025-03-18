// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused, reason = "Development")]
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    ops::{Range, RangeInclusive},
};

use smallvec::SmallVec;
use vello::kurbo::{Point, Size, Vec2};
use winit::keyboard::{Key, NamedKey};

use crate::core::{
    BoxConstraints, PointerEvent, PropertiesMut, PropertiesRef, TextEvent, Widget, WidgetMut,
    WidgetPod,
};

use super::CrossAxisAlignment;

pub struct VirtualScrollAction {
    pub old_active: Range<i64>,
    pub target: Range<i64>,
}

/// We assume that by default, virtual scrolling items are at least ~30 logical pixels tall (two lines of text + a bit).
/// Because we load the visible page, and a page above and below that, a safety margin of 2 effectively applies.
///
/// We err in the positive direction because we expect to end up in a fixed-point loop, so if we have loaded
/// too few items, that will be sorted relatively quickly.
const DEFAULT_MEAN_ITEM_HEIGHT: f64 = 60.;

/// The model of this virtual scrolling is thus:
///
/// 1) The `VirtualScroll` "knows" what item "index" it is using as an anchor
/// 2) It "knows" how far away from that anchor it is
/// 3) It keeps the item which is focused in its awareness at all times
/// 4) It knows how tall any item it requests is, only once layout happens
/// 5) It importantly does not *care* about scrollbars. This has several big advantages:
///    - It allows for "infinite" virtual up and down-scrolling
///    - It allows for much more control by users of how the scrolling happens
///
// TODO: Should `W` be a generic, or just always `dyn Widget`?
pub struct VirtualScroll<W: Widget + ?Sized> {
    /// The range of items in the "id" space which are able to be used.
    ///
    /// This is used to cap scrolling; items outside of this range will never be loaded[^1][^2][^3].
    /// For example, in an email program, this would be `[id_of_most_recent_email]..=[id_of_oldest_email]`
    /// (note that the id of the oldest email might not be known; as soon as it is known, `id_of_oldest_email`
    /// can be set).
    ///
    /// The default is `i64::MIN..i64::MAX`. Note that this *is* exclusive of the item with id `i64::MAX`.
    /// That additional item being missing allows for using half-open ranges in all of this code,
    /// which makes our lives much easier.
    ///
    /// [^1]: The exact interaction with a "drag down to refresh" feature has not been scrutinised.
    /// [^2]: Currently, we lock the bottom of the range to the bottom of the final item. This should be configurable.
    /// [^3]: Behaviour when the range is shrunk to something containing the active range has not been considered.
    // TODO: We should check that this is not "backwards" (normal empty is fine).
    valid_range: Range<i64>,

    /// The range in the id space which is "active", i.e. which the virtual scrolling has decided
    /// are in the range of the viewport and should be shown on screen.
    /// Note that `items` is not necessarily dense in these; that is, if an
    /// item has not been provided by the application, we don't fall over.
    /// This is still an invalid state, but we handle it as well as we can.
    active_range: Range<i64>,

    /// All children of the virtual scroller.
    items: HashMap<i64, WidgetPod<W>>,
    // TODO: Handle focus even if the focused item scrolls off-screen.
    // TODO: Maybe this should be the focused items and its two neighbours, so tab focusing works?
    // focused_item: Option<(i64, WidgetPod<W>)>,

    // Question: For a given scroll position, should the anchor always be the same?
    // Answer: Let's say yes for now, and re-evaluate if it becomes necessary.
    //  - Reason to not have this is that it adds some potential worst-case performance issues if scrolling up/down
    anchor_index: i64,
    /// The amount the user has scrolled from the anchor point, in logical pixels.
    scroll_offset_from_anchor: f64,

    /// The average height of items, determined experimentally.
    /// This is used if there are no items to determine the mean item height otherwise. This approach means:
    /// 1) For the easy case where every item is the same height (e.g. email), we get the right answer
    /// 2) For slightly harder cases, we get as sensible a result as is reasonable, without requiring a complex API
    ///    to get the needed information.
    mean_item_height: f64,

    /// The height of the current anchor.
    /// Used to determine if scrolling will require a relayout (because the anchor will have changed if the user has scrolled past it).
    anchor_height: f64,

    /// The available width in the last layout call, used so that layout of children can be skipped if it won't have changed.
    old_width: f64,

    /// We don't want to spam warnings about not being dense, but we want the user to be aware of it.
    warned_not_dense: bool,
}

impl<W: Widget + ?Sized> std::fmt::Debug for VirtualScroll<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualScroll")
            .field("valid_range", &self.valid_range)
            .field("active_range", &self.active_range)
            .field("items", &self.items.keys().collect::<Vec<_>>())
            .field("anchor_index", &self.anchor_index)
            .field("scroll_offset_from_anchor", &self.scroll_offset_from_anchor)
            .field("mean_item_height", &self.mean_item_height)
            .field("anchor_height", &self.anchor_height)
            .field("old_width", &self.old_width)
            .field("warned_not_dense", &self.warned_not_dense)
            .finish()
    }
}

impl<W: Widget + ?Sized> VirtualScroll<W> {
    pub fn new(initial_anchor: i64) -> Self {
        Self {
            // TODO: Allow configuring this.
            valid_range: i64::MIN..i64::MAX,
            // This range starts intentionally empty, as no items have been loaded.
            active_range: initial_anchor..initial_anchor,
            items: HashMap::default(),
            anchor_index: initial_anchor,
            scroll_offset_from_anchor: 0.0,
            mean_item_height: DEFAULT_MEAN_ITEM_HEIGHT,
            anchor_height: DEFAULT_MEAN_ITEM_HEIGHT,
            old_width: f64::NAN,
            warned_not_dense: false,
        }
    }

    pub fn remove_child(this: &mut WidgetMut<Self>, idx: i64) {
        let child = this.widget.items.remove(&idx);
        if let Some(child) = child {
            this.ctx.remove_child(child);
        } else {
            tracing::warn!("Tried to remove child which has already been removed");
        }
    }

    pub fn add_child(this: &mut WidgetMut<Self>, idx: i64, mut child: WidgetPod<W>) {
        this.ctx.children_changed();
        // let active = this.widget.active_range.contains(&idx);
        // this.ctx.set_stashed(&mut child, !active);
        if this.widget.items.insert(idx, child).is_some() {
            tracing::warn!("Tried to add child {idx} twice to VirtualScroll");
        };
    }

    pub fn overwrite_anchor(this: &mut WidgetMut<Self>, idx: i64) {
        this.widget.anchor_index = idx;
        this.widget.scroll_offset_from_anchor = 0.;
        this.ctx.request_layout();
    }

    fn post_scroll(&mut self, ctx: &mut crate::core::EventCtx<'_>) {
        if self.anchor_index + 1 >= self.valid_range.end {
            self.cap_scroll_range_down(self.anchor_height, ctx.size().height);
        }
        if self.anchor_index <= self.valid_range.start {
            self.cap_scroll_range_up();
        }
        if self.scroll_offset_from_anchor < 0.
            || self.scroll_offset_from_anchor >= self.anchor_height
        {
            ctx.request_layout();
        }
        ctx.request_compose();
    }

    /// Lock scrolling so that:
    /// 1) Every part of the last item can be seen.
    /// 2) The last item never scrolls completely out of view (currently, the bottom of the last item can be halfway down the screen)
    ///
    /// Ideally, this would be configurable (so that e.g. the bottom of the last item aligns with
    /// the bottom of the viewport), but that requires more care, since it effectively changes what the last valid anchor is.
    fn cap_scroll_range_down(&mut self, anchor_height: f64, viewport_height: f64) {
        self.scroll_offset_from_anchor = self
            .scroll_offset_from_anchor
            // TODO: There is still some jankiness when scrolling into the last item; this is for reasons unknown.
            .min((anchor_height - viewport_height / 2.).max(0.0));
    }
    fn cap_scroll_range_up(&mut self) {
        self.scroll_offset_from_anchor = self.scroll_offset_from_anchor.max(0.0);
    }
}

impl<W: Widget + ?Sized> Widget for VirtualScroll<W> {
    fn layout(
        &mut self,
        ctx: &mut crate::core::LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &crate::core::BoxConstraints,
    ) -> vello::kurbo::Size {
        let viewport_size = bc.max();
        let child_constraints_changed = viewport_size.width != self.old_width;
        self.old_width = viewport_size.width;
        ctx.set_clip_path(viewport_size.to_rect());
        let child_bc = BoxConstraints::new(
            Size {
                width: viewport_size.width,
                height: 0.,
            },
            Size {
                width: viewport_size.width,
                // TODO: Infinite constraints are... not ideal
                height: f64::INFINITY,
            },
        );
        // The number of loaded items before the anchor
        let mut count_before_anchor = 0;
        let mut height_before_anchor = 0.;
        // The height of all loaded items after the anchor.
        // Note that this includes the height of the anchor itself.
        let mut height_after_anchor = 0.;
        let mut count = 0_u64;
        let mut first_item = i64::MAX;
        let mut last_item = i64::MIN;

        // Calculate the sizes of all children
        for (idx, child) in &mut self.items {
            if !self.active_range.contains(idx) {
                if cfg!(debug_assertions) && ctx.child_needs_layout(child) {
                    unreachable!(
                        "Mutate pass didn't run before layout, breaking assumptions @ {idx}."
                    )
                }
                // This item is stashed, and so can't be used.
                // (If we decide we want to use it, we'll do so by re-enabling
                // it in a following mutate pass).
                ctx.skip_layout(child);
                continue;
            }

            let resulting_size = if child_constraints_changed || ctx.child_needs_layout(child) {
                ctx.run_layout(child, &child_bc)
            } else {
                ctx.skip_layout(child);
                ctx.child_size(child)
            };
            if *idx < self.anchor_index {
                count_before_anchor += 1;
                height_before_anchor += resulting_size.height;
            } else {
                height_after_anchor += resulting_size.height;
            }
            count += 1;
        }

        let mean_item_height = if count > 0 {
            (height_before_anchor + height_after_anchor) / count as f64
        } else {
            self.mean_item_height
        };
        let mean_item_height = if !mean_item_height.is_finite() {
            tracing::warn!(
                "Got a non-finite mean item height {mean_item_height} in virtual scrolling"
            );
            DEFAULT_MEAN_ITEM_HEIGHT
        } else {
            mean_item_height
        };
        self.mean_item_height = mean_item_height;

        // Determine the new anchor
        loop {
            if self.scroll_offset_from_anchor < 0. {
                if self.anchor_index <= self.valid_range.start {
                    self.anchor_index = self.valid_range.start;
                    self.cap_scroll_range_up();
                    break;
                }
                self.anchor_index -= 1;
                let new_anchor_height = if self.active_range.contains(&self.anchor_index) {
                    let new_anchor = self.items.get(&self.anchor_index);
                    if let Some(new_anchor) = new_anchor {
                        ctx.child_size(new_anchor).height
                    } else {
                        // We don't treat missing items inside the set of loaded items as having a height.
                        // This avoids potential infinite loops (from adding a new
                        // item increasing the mean item size, causing that new item to become unloaded)
                        0.0
                    }
                } else {
                    // In theory, even for inactive items which haven't been removed, we could
                    // get their prior height.
                    // However, we choose not to do this to make behaviour predictable; we don't
                    // want there to be any advantage to not removing items which should be removed.
                    mean_item_height
                };

                self.scroll_offset_from_anchor += new_anchor_height;
                height_before_anchor -= new_anchor_height;
            } else {
                let last_item = self.anchor_index + 1 >= self.valid_range.end;
                if last_item {
                    self.anchor_index = self.valid_range.end - 1;
                }
                let anchor_height = if self.active_range.contains(&self.anchor_index) {
                    let current_anchor = self.items.get(&self.anchor_index);
                    if let Some(anchor_pod) = current_anchor {
                        ctx.child_size(anchor_pod).height
                    } else {
                        0.0
                    }
                } else {
                    mean_item_height
                };
                if last_item {
                    self.cap_scroll_range_down(anchor_height, viewport_size.height);
                    break;
                }
                if self.scroll_offset_from_anchor >= anchor_height {
                    self.anchor_index += 1;
                    self.scroll_offset_from_anchor -= anchor_height;
                    height_before_anchor += anchor_height;
                } else {
                    break;
                }
            }
        }
        self.anchor_height = if let Some(anchor) = self.items.get(&self.anchor_index) {
            ctx.child_size(anchor).height
        } else {
            mean_item_height
        };

        // Load a page and a half above the screen
        let cutoff_up = viewport_size.height * 1.5;
        // Load a page and a half below the screen (note that this cutoff "includes" the screen)
        let cutoff_down = viewport_size.height * 2.5;

        let mut item_crossing_top = None;
        let mut item_crossing_bottom = self.active_range.start;
        // TODO: How can we even plausibly handle arbitrary transforms?
        // Answer: We clip each child to a box at most (say) 20% taller than their layout box.
        let mut y = -height_before_anchor;
        let mut was_dense = true;
        // We lay all of the active items out (even though some of them will be made inactive
        // after layout is done)
        for idx in self.active_range.clone() {
            if y <= -cutoff_up {
                item_crossing_top = Some(idx);
            }
            if y <= cutoff_down {
                item_crossing_bottom = idx;
            }
            let item = self.items.get_mut(&idx);
            if let Some(item) = item {
                let size = ctx.child_size(item);
                ctx.place_child(item, Point::new(0., y));
                // TODO: Padding/gap?
                y += size.height;
            } else {
                was_dense = false;
                // We expect the virtual scrolling to be dense; we are designed
                // to handle the non-dense case gracefully, but it is a bug in your
                // component/app if the results are not dense.
                if !self.warned_not_dense {
                    self.warned_not_dense = true;
                    tracing::error!(
                        "Virtual Scrolling items in {:?} ({}) not dense.\n\
                        Expected to be dense in {:?}, but missing {idx}",
                        ctx.widget_id(),
                        self.type_name(),
                        self.active_range,
                    );
                }
            }
        }
        if was_dense {
            // For each time we have the falling edge of becoming not dense, we want to warn.
            self.warned_not_dense = false;
        }
        let target_range = if self.active_range.contains(&self.anchor_index) {
            let start = if let Some(item_crossing_top) = item_crossing_top {
                item_crossing_top
            } else {
                let number_needed =
                    ((cutoff_up - height_before_anchor) / mean_item_height).ceil() as i64;
                self.active_range.start - number_needed
            };
            let end = if y > cutoff_down {
                item_crossing_bottom + 1
            } else {
                // `y` is the bottom of the bottommost loaded item
                let number_needed = ((cutoff_down - y) / mean_item_height).ceil() as i64;
                self.active_range.end + number_needed
            };
            start..end
        } else {
            // We've jumped a huge distance in view space (see `Self::overwrite_anchor`)
            // Handle that sanely.
            let start = self.anchor_index - (cutoff_up / mean_item_height).ceil() as i64;
            let end = self.anchor_index + (cutoff_down / mean_item_height).ceil() as i64;
            start..end
        };

        // Avoid requesting invalid items by clamping to the valid range
        let target_range = target_range
            .start
            // target_range.start is inclusive whereas valid_range.end is exclusive; convert between the two.
            .clamp(self.valid_range.start, self.valid_range.end - 1)
            ..target_range
                .end
                .clamp(self.valid_range.start, self.valid_range.end);

        if self.active_range != target_range {
            let previous_active = self.active_range.clone();
            self.active_range = target_range;

            {
                let previous_active = previous_active.clone();
                // Stash all previously active widgets which are still loaded.
                // This is needed for the case where there is a second iteration of passes (incl. layout)
                // of the normal passes *before* the action gets handled.
                // This is done this way because `LayoutCtx::set_stashed` is documented to be planned for removal.
                // Note that this will never unstash items; those must be removed and re-added.
                // N.B. this could break with an adversarial set of circumstances, because:
                // - `mutate_self_later` doesn't actually force a new run of the rewrite passes
                //    (https://xi.zulipchat.com/#narrow/channel/354396-xilem/topic/Virtual.20scrolling.20list.20redux/near/505728926); AND
                // - `mutate_self_later` runs before the Update Tree (which adds new widgets added by the action); AND
                // - `set_stashed` panics if the item hasn't been "recorded", i.e. it's a new item since the last time update tree ran.
                // Therefore, an adversarial driver could force this code to panic by adding a widget which is in the old set, which won't
                // be valid to call `set_stashed` on.
                // However, there's no other way to encode this operation at the moment.
                ctx.mutate_self_later(move |mut this| {
                    // It's critical that nothing here produces a layout pass, otherwise we would get into an infinite loop
                    let mut this = this.downcast::<Self>();
                    for idx in opt_iter_difference(&previous_active, &this.widget.active_range) {
                        let item = this.widget.items.get_mut(&idx);
                        if let Some(item) = item {
                            this.ctx.set_stashed(item, true);
                        }
                    }
                });
            }

            ctx.submit_action(crate::core::Action::Other(Box::new(VirtualScrollAction {
                old_active: previous_active,
                target: self.active_range.clone(),
            })));
        }
        // TODO: We should still try and find a way to detect infinite loops;
        // our pattern for this should avoid it, but if that assessment is wrong, the outcome would be very bad
        // (a driver which didn't correctly set `valid_range` would be one cause).

        // In theory, if we have loaded all of the items in self.valid_range, we can tell the platform that this is our full size.
        // Practically, that is such a rare case that it isn't worth doing.
        viewport_size
    }

    fn compose(&mut self, ctx: &mut crate::core::ComposeCtx) {
        let translation = Vec2 {
            x: 0.,
            y: -self.scroll_offset_from_anchor,
        };
        for idx in self.active_range.clone() {
            if let Some(child) = self.items.get_mut(&idx) {
                ctx.set_child_scroll_translation(child, translation);
            }
        }
    }

    fn accessibility_role(&self) -> accesskit::Role {
        // TODO: accesskit::Role::ScrollView ?
        accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        ctx: &mut crate::core::AccessCtx,
        _props: &PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
        // TODO: Better virtual scrolling accessibility
        // Intended as a follow-up collaboration with Matt
        node.set_clips_children();
    }

    fn on_access_event(
        &mut self,
        ctx: &mut crate::core::EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &crate::core::AccessEvent,
    ) {
        // TODO: Handle scroll events
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut crate::core::EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &crate::core::PointerEvent,
    ) {
        const SCROLLING_SPEED: f64 = 10.0;

        let portal_size = ctx.size();

        match event {
            PointerEvent::MouseWheel(delta, _) => {
                let delta = delta.y * -SCROLLING_SPEED;
                self.scroll_offset_from_anchor += delta;
                self.post_scroll(ctx);
            }
            _ => (),
        }

        // TODO: Handle scroll wheel
    }
    fn accepts_pointer_interaction(&self) -> bool {
        // We handle e.g. scroll wheel events
        true
    }

    fn children_ids(&self) -> smallvec::SmallVec<[crate::core::WidgetId; 16]> {
        self.items.values().map(|pod| pod.id()).collect()
    }

    fn register_children(&mut self, ctx: &mut crate::core::RegisterCtx) {
        for child in self.items.values_mut() {
            ctx.register_child(child);
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut crate::core::PaintCtx,
        _props: &PropertiesRef<'_>,
        _scene: &mut vello::Scene,
    ) {
    }

    fn on_text_event(
        &mut self,
        ctx: &mut crate::core::EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::KeyboardKey(key_event, modifiers_state) => {
                // To get to this state, you currently need to press "tab" to focus this widget in the example.
                if key_event.state.is_pressed() {
                    let delta = 20000.;
                    if matches!(key_event.logical_key, Key::Named(NamedKey::PageDown)) {
                        self.scroll_offset_from_anchor += delta;
                        self.post_scroll(ctx);
                    }
                    if matches!(key_event.logical_key, Key::Named(NamedKey::PageUp)) {
                        self.scroll_offset_from_anchor -= delta;
                        self.post_scroll(ctx);
                    }
                }
            }
            _ => {}
        }
        // Maybe? Handle pagedown? or something like escape for keyboard focus to escape the virtual list
    }
    fn accepts_text_input(&self) -> bool {
        false
    }

    fn update(
        &mut self,
        ctx: &mut crate::core::UpdateCtx,
        _props: &mut PropertiesMut<'_>,
        event: &crate::core::Update,
    ) {
        match event {
            crate::core::Update::WidgetAdded => {}
            crate::core::Update::DisabledChanged(_) => {}
            crate::core::Update::StashedChanged(_) => {}
            crate::core::Update::RequestPanToChild(rect) => {} // TODO,
            crate::core::Update::HoveredChanged(_) => {}
            crate::core::Update::ChildHoveredChanged(_) => {}
            crate::core::Update::FocusChanged(_) => {}
            crate::core::Update::ChildFocusChanged(_) => {
                // TODO: We won't actually get this event if *which* child element is focused changes...
                // In fact, there's *no* reliable way to detect that, which makes proper focus management impossible
            }
        }
    }
    fn accepts_focus(&self) -> bool {
        // TODO: Maybe we should make this true, to properly capture tab?
        true
    }

    // TODO: Optimise using binary search?
    // fn find_widget_at_pos(..);

    fn get_debug_text(&self) -> Option<String> {
        Some(format!("{self:#?}"))
    }
}

/// Optimisation for:
/// ```
/// let old_range = 0i64..10;
/// let new_range = 0i64..10;
/// for idx in old_range {
///     if !new_range.contains(&idx) {
///         // ...
///     }
/// }
/// ```
/// as an iterator
fn opt_iter_difference(
    old_range: &Range<i64>,
    new_range: &Range<i64>,
) -> std::iter::Chain<Range<i64>, Range<i64>> {
    (old_range.start..(new_range.start.min(old_range.end)))
        .chain(new_range.end.max(old_range.start)..old_range.end)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use parley::StyleProperty;
    use vello::kurbo::Size;

    use crate::{
        assert_render_snapshot,
        core::{Widget, WidgetMut, WidgetPod},
        testing::{TestHarness, widget_ids},
        widgets::{Label, VirtualScroll, VirtualScrollAction},
    };

    use super::opt_iter_difference;

    #[test]
    #[expect(
        clippy::reversed_empty_ranges,
        reason = "Testing technically possible behaviour"
    )]
    fn opt_iter_difference_equiv() {
        let ranges = [
            5..10,
            7..15,
            -10..7,
            // Negative ranges are empty; those should be respected.
            // The optimised version does actually do more than is needed if the new range is negative
            // However, we don't expect negative ranges to be common (only supported for robustness), so
            // we don't care if they aren't handled as performantly as possible, so long as it doesn't miss anything
            20..10,
            12..17,
        ];
        for old_range in &ranges {
            for new_range in &ranges {
                let opt_result = opt_iter_difference(old_range, new_range).collect::<HashSet<_>>();
                let mut naive_result = HashSet::new();
                for idx in old_range.clone() {
                    if !new_range.contains(&idx) {
                        naive_result.insert(idx);
                    }
                }
                assert_eq!(
                    opt_result, naive_result,
                    "The optimised version of differences should be equivalent to the trivially \
                    correct method, but wasn't for {old_range:?} and {new_range:?}"
                );
            }
        }
    }

    #[test]
    fn sensible_driver() {
        let [item_3_id, item_13_id] = widget_ids();

        type ScrollContents = Label;

        let widget = VirtualScroll::<ScrollContents>::new(0);

        let mut harness = TestHarness::create_with_size(widget, Size::new(100., 200.));
        let virtual_scroll_id = harness.root_widget().id();
        fn driver(action: VirtualScrollAction, mut scroll: WidgetMut<'_, VirtualScroll<Label>>) {
            for idx in action.old_active {
                VirtualScroll::remove_child(&mut scroll, idx);
            }
            for idx in action.target {
                VirtualScroll::add_child(
                    &mut scroll,
                    idx,
                    WidgetPod::new(
                        Label::new(format!("{idx}")).with_style(StyleProperty::FontSize(30.)),
                    ),
                );
            }
        }

        drive_to_fixpoint::<ScrollContents>(&mut harness, virtual_scroll_id, driver);
        assert_render_snapshot!(harness, "virtual_scroll_basic");
        harness.edit_widget(virtual_scroll_id, |mut portal| {
            let mut scroll = portal.downcast::<VirtualScroll<ScrollContents>>();
            VirtualScroll::overwrite_anchor(&mut scroll, 100);
        });
        drive_to_fixpoint::<ScrollContents>(&mut harness, virtual_scroll_id, driver);
        assert_render_snapshot!(harness, "virtual_scroll_moved");
        // let item_3_rect = harness.get_widget(item_3_id).ctx().local_layout_rect();
        // harness.edit_root_widget(|mut portal| {
        //     let mut portal = portal.downcast::<Portal<Flex>>();
        //     Portal::pan_viewport_to(&mut portal, item_3_rect);
        // });

        // assert_render_snapshot!(harness, "button_list_scroll_to_item_3");

        // let item_13_rect = harness.get_widget(item_13_id).ctx().local_layout_rect();
        // harness.edit_root_widget(|mut portal| {
        //     let mut portal = portal.downcast::<Portal<Flex>>();
        //     Portal::pan_viewport_to(&mut portal, item_13_rect);
        // });

        // assert_render_snapshot!(harness, "button_list_scroll_to_item_13");
    }

    fn drive_to_fixpoint<T: Widget + ?Sized>(
        harness: &mut TestHarness,
        virtual_scroll_id: crate::core::WidgetId,
        mut f: impl FnMut(VirtualScrollAction, WidgetMut<'_, VirtualScroll<T>>),
    ) {
        let mut iteration = 0;
        let mut old_active = None;
        loop {
            iteration += 1;
            if iteration > 10 {
                panic!("Took too long to reach fixpoint");
            }
            let Some((action, id)) = harness.pop_action() else {
                break;
            };
            assert_eq!(
                id, virtual_scroll_id,
                "Only widget in tree should give action"
            );
            assert_eq!(harness.pop_action(), None);
            let crate::core::Action::Other(action) = action else {
                unreachable!()
            };
            let action = action.downcast::<VirtualScrollAction>().unwrap();
            if let Some(old_active) = old_active.take() {
                assert_eq!(action.old_active, old_active);
            }
            old_active = Some(action.target.clone());
            // This could happen iff the valid range is empty, which is case I've not reasoned about yet.
            assert!(!action.target.is_empty());

            harness.edit_widget(virtual_scroll_id, |mut portal| {
                let mut scroll = portal.downcast::<VirtualScroll<T>>();
                f(*action, scroll);
            });
        }
    }
}
