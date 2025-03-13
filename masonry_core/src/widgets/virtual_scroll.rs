// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused, reason = "Development")]
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    ops::Range,
};

use smallvec::SmallVec;
use vello::kurbo::{Point, Size, Vec2};

use crate::core::{BoxConstraints, PropertiesMut, PropertiesRef, Widget, WidgetPod};

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
pub struct VirtualScroll<W: Widget + ?Sized> {
    /// The range of items in the "id" space which are able to be used.
    ///
    /// This is used to cap scrolling; items outside of this range will never be loaded[^1][^2][^3].
    /// For example, in an email program, this would be `[id_of_most_recent_email]..=[id_of_oldest_email]`
    /// (note that the id of the oldest email might not be known; as soon as it is known, `id_of_oldest_email`
    /// can be set).
    ///
    /// The default is `i64::MIN..i64::MAX`
    ///
    /// Items outside of this range will be stashed.
    ///
    /// [^1]: The exact interaction with a "drag down to refresh" feature has not been scrutinised.
    /// [^2]: Currently, we lock the bottom of the range to the bottom of the final item. This should be configurable.
    /// [^3]: Behaviour when the range is shrunk to something containing the active range has not been considered.
    valid_range: Range<i64>,

    /// The range in the id space which is "active", i.e. which the virtual scrolling controller has the
    /// ability to lay out. Note that items is not necessarily dense in these; that is, if an
    /// item has not been provided by the application, we avoid falling over.
    active_range: Range<i64>,

    /// The range in the id space which the virtual scrolling controller *wants* to layout.
    /// Items which are in `active_range` but not in this range will be stashed in the mutate pass.
    target_range: Range<i64>,

    /// All children of the virtual scroller.
    // TODO: Does this need to be a `BTreeMap`, or maybe a sparse `VecDeque` (for quicker in-order iteration)
    items: HashMap<i64, WidgetPod<W>>,
    // TODO: Handle focus even if the focused item scrolls off-screen.
    // TODO: Maybe this should be the focused items and its two neighbours, so tab focusing works?
    // focused_item: Option<(i64, WidgetPod<W>)>,

    // Question: For a given scroll position, should the anchor always be the same?
    // Answer: Let's say yes for now, and re-evaluate if it becomes necessary.
    //  - Reason to not have this is that it adds some potential worst-case performance issues if scrolling up/down
    anchor_index: i64,
    // TODO: Use a fixed-point scheme, because adding and subtracting amounts could change the scale, theoretically causing visible jumps.
    // This will likely wait for layout to do something similar
    scroll_offset_from_anchor: f64,

    /// The average height of items, determined experimentally.
    /// This is used if there are no items to determine the mean item height otherwise. This approach means:
    /// 1) For the easy case where every item is the same height (e.g. email), we get the right answer
    /// 2) For slightly harder cases, we get as sensible a result as is reasonable, without requiring a complex API
    ///    to get the needed information.
    mean_item_height: f64,

    /// The height of the current anchor.
    /// Used to determine if scrolling will require a relayout (because the anchor has changed).
    anchor_height: f64,

    /// The available width in the last layout call, used so that layout of children can be skipped if it won't have changed.
    old_width: f64,
}

impl<W: Widget> Widget for VirtualScroll<W> {
    fn layout(
        &mut self,
        ctx: &mut crate::core::LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &crate::core::BoxConstraints,
    ) -> vello::kurbo::Size {
        let viewport_size = bc.max();
        let child_constraints_changed = viewport_size.width == self.old_width;
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
                    unreachable!("Mutate pass didn't run before layout, breaking assumptions.")
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

        let items_after_anchor: u64 = (self.active_range.end - self.anchor_index)
            .try_into()
            .expect("Anchor is within the active range.");
        let items_before_anchor: u64 = (self.anchor_index - self.active_range.start)
            .try_into()
            .expect("Anchor is within the active range.");

        // The total height of all active items which are after the current anchor
        // Note that this includes our best estimate of the
        // let height_after_anchor = height_after_anchor
        //     + (items_after_anchor - (count - count_before_anchor)) as f64 * mean_item_height;

        // The total height of all active items which are before the current anchor
        // Note that this uses the mean height for any items not currently loaded (i.e. provided by the user)
        // In most cases, these items being missing is a bug in the driver, but we avoid falling down anyway.
        let mut height_before_anchor = height_before_anchor
            + (items_before_anchor - count_before_anchor) as f64 * mean_item_height;
        // If we've significantly out-ran the virtual scrolling (as a loose heuristic), calculate a new anchor using a heuristic
        // and drop all currently loaded items.
        // if (self.scroll_offset_from_anchor
        //     > height_after_anchor + 3. * viewport_size.height + 3000.)
        //     || (self.scroll_offset_from_anchor
        //         < -height_before_anchor - 3. * viewport_size.height - 3000.)
        // {
        // let items = core::mem::take(&mut self.current_items);
        // // TODO: We *probably* want to slide down the items.
        // for (idx, mut item) in items {
        //     ctx.skip_layout(&mut item);
        //     self.items_pending_removal.insert(idx, item);
        //     removals.push(idx);
        // }
        // if mean_item_size.is_finite() {
        //     let diff_count = self.scroll_offset_from_anchor / mean_item_size;
        //     let diff_count = diff_count.floor();
        //     self.anchor_index =
        //         (self.anchor_index + diff_count as i64).clamp(self.first_item, self.last_item);
        //     self.scroll_offset_from_anchor -= diff_count * mean_item_size;
        // } else {
        //     debug_assert_eq!(
        //         count, 0,
        //         "Assumption: If the division produced an infinite result, it's because of a divide by zero."
        //     );
        //     // TODO: How can we handle this sanely? We're scrolled very far down (3 pages + 3000 logical pixels),
        //     // but we don't have any idea how big the items are; that could be within a single item.
        //     // I guess we need to just wait for the anchor to be loaded?
        // }
        // } else

        // Determine the new anchor

        // let previous_anchor = self.anchor_index;
        loop {
            if self.scroll_offset_from_anchor < 0. {
                if self.anchor_index <= self.valid_range.start {
                    // TODO: Is this the right time to do this clamping?
                    self.anchor_index = self.valid_range.start;
                    // Don't scroll above the topmost item
                    self.scroll_offset_from_anchor = 0.;
                    break;
                }
                self.anchor_index -= 1;
                let new_anchor_height = if self.active_range.contains(&self.anchor_index) {
                    let new_anchor = self.items.get(&self.anchor_index);
                    if let Some(new_anchor) = new_anchor {
                        ctx.child_size(new_anchor).height
                    } else {
                        mean_item_height
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
                let anchor_height = if self.active_range.contains(&self.anchor_index) {
                    let current_anchor = self.items.get(&self.anchor_index);
                    if let Some(anchor_pod) = current_anchor {
                        ctx.child_size(anchor_pod).height
                    } else {
                        mean_item_height
                    }
                } else {
                    mean_item_height
                };
                if self.scroll_offset_from_anchor > anchor_height {
                    if self.anchor_index >= self.valid_range.end {
                        // TODO: Is this the right time to do this clamping?
                        self.anchor_index = self.valid_range.end;
                        self.scroll_offset_from_anchor = self
                            .scroll_offset_from_anchor
                            // Lock scrolling to be at most a page below the last item
                            .max(anchor_height + viewport_size.height);
                        break;
                    }
                    self.anchor_index += 1;
                    self.scroll_offset_from_anchor -= anchor_height;
                    height_before_anchor += anchor_height;
                } else {
                    break;
                }
            }
        }

        // TODO: I *suspect* we need to do something else?
        // self.anchor_index = new_anchor;

        // TODO: How can we even plausibly handle arbitrary transforms?
        // Answer: We clip each child to a box at most (say) 20% taller than their layout box.
        let mut y = -height_before_anchor;
        // We lay all of the active items out, even if some of them will be outside the requested range.
        for idx in self.active_range.clone() {
            let item = self.items.get_mut(&idx);
            if let Some(item) = item {
                let size = ctx.child_size(item);
                ctx.place_child(item, Point::new(0., y));
                // TODO: Padding/gap?
                y += size.height;
            } else {
                y += mean_item_height;
            }
        }

        let target_range = if mean_item_height.is_finite() {
            let up = (viewport_size.height * 1.5 / mean_item_height).ceil() as i64 + 1;
            let down = (viewport_size.height * 2.5 / mean_item_height).ceil() as i64 + 1;
            (self.anchor_index - up)..(self.anchor_index + down)
        } else {
            ((self.anchor_index - 2)..(self.anchor_index + 5))
        };
        self.target_range = target_range;

        ctx.mutate_self_later(move |mut this| {
            let mut this = this.downcast::<Self>();
            for idx in this.widget.active_range.clone() {
                if !this.widget.target_range.contains(&idx) {
                    let item = this.widget.items.get_mut(&idx);
                    if let Some(item) = item {
                        this.ctx.set_stashed(item, true);
                    }
                }
            }
            for idx in this.widget.target_range.clone() {
                if !this.widget.active_range.contains(&idx) {
                    let item = this.widget.items.get_mut(&idx);
                    if let Some(item) = item {
                        // TODO: Are there reasonable cases where this happens?
                        // A widget that scrolls a different widget into view in response to being stashed?!
                        tracing::debug!(
                            "Found item {:?} (index {idx}) which should have been removed by driver, in `VirtualScroll`", item.id()
                        );
                        this.ctx.set_stashed(item, false);
                    }
                }
            }
        });
        ctx.submit_action(crate::core::Action::Other(Box::new(VirtualScrollAction {
            old_active: self.active_range.clone(),
            target: self.target_range.clone(),
        })));
        viewport_size
    }

    fn compose(&mut self, ctx: &mut crate::core::ComposeCtx) {
        let translation = Vec2 {
            x: 0.,
            y: self.scroll_offset_from_anchor,
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
        event: &crate::core::TextEvent,
    ) {
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
            crate::core::Update::FocusChanged(_) => {
                if cfg!(debug_assertions) {
                    unreachable!("VirtualScroll can't be focused")
                }
            }
            crate::core::Update::ChildFocusChanged(_) => {
                // TODO: We won't actually get this event if *which* child element is focused changes...
                // In fact, there's *no* reliable way to detect that, which makes proper focus management impossible
            }
        }
    }
    fn accepts_focus(&self) -> bool {
        // TODO: Maybe we should make this true, to properly capture tab?
        false
    }

    // TODO: Optimise using binary search?
    // fn find_widget_at_pos(..);
}
