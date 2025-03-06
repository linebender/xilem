// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused, reason = "Development")]
use std::collections::{BTreeMap, HashMap, VecDeque};

use smallvec::SmallVec;
use vello::kurbo::{Point, Size, Vec2};

use crate::core::{BoxConstraints, Widget, WidgetPod};

use super::CrossAxisAlignment;

pub struct VirtualScrollAction {
    add: SmallVec<[i64; 8]>,
    remove: SmallVec<[i64; 8]>,
}

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
    /// Must be less than `last_item`
    first_item: i64,
    last_item: i64,
    current_items: BTreeMap<i64, WidgetPod<W>>,
    items_pending_removal: HashMap<i64, WidgetPod<W>>,
    // TODO: Handle focus even if the focused item scrolls off-screen.
    // TODO: Maybe this should be the focused items and its two neighbours, so tab focusing works?
    // focused_item: Option<(i64, WidgetPod<W>)>,

    // Question: For a given scroll position, should the anchor always be the same?
    // Answer: Let's say yes for now, and re-evaluate if it becomes necessary.
    //  - Reason to not have this is that it adds some pathologies to an "up/down" movement
    anchor_index: i64,
    // TODO: Use a fixed-point scheme, because
    // This will likely wait for layout to do something similar
    scroll_offset_from_anchor: f64,

    old_width: f64,
}

impl<W: Widget> Widget for VirtualScroll<W> {
    fn layout(
        &mut self,
        ctx: &mut crate::core::LayoutCtx,
        bc: &crate::core::BoxConstraints,
    ) -> vello::kurbo::Size {
        for child in self.items_pending_removal.values_mut() {
            ctx.skip_layout(child);
        }
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
        let mut height_before_anchor = 0.;
        // The height of all loaded items after the anchor.
        // Note that this includes the height of the anchor itself.
        let mut height_after_anchor = 0.;
        for (idx, child) in &mut self.current_items {
            let resulting_size = if child_constraints_changed || ctx.child_needs_layout(child) {
                ctx.run_layout(child, &child_bc)
            } else {
                ctx.skip_layout(child);
                ctx.child_size(child)
            };
            if *idx < self.anchor_index {
                height_before_anchor += resulting_size.height;
            } else {
                height_after_anchor += resulting_size.height;
            }
        }

        let count = self.current_items.len();
        let mean_item_size = (height_before_anchor + height_after_anchor) / count as f64;

        // If we've significantly out-ran the virtual scrolling (as a loose heuristic), calculate a new anchor using a heuristic
        // and drop all currently loaded items.
        if (self.scroll_offset_from_anchor
            > height_after_anchor + 3. * viewport_size.height + 3000.)
            || (self.scroll_offset_from_anchor
                < -height_before_anchor - 3. * viewport_size.height - 3000.)
        {
            let items = core::mem::take(&mut self.current_items);
            for (idx, mut item) in items {
                ctx.skip_layout(&mut item);
                self.items_pending_removal.insert(idx, item);
            }
            // TODO: Consolidate these mutations
            ctx.mutate_self_later(|mut this| {
                let mut this = this.downcast::<Self>();
                for element in this.widget.items_pending_removal.values_mut() {
                    this.ctx.set_stashed(element, true);
                }
            });
            if mean_item_size.is_finite() {
                let diff_count = self.scroll_offset_from_anchor / mean_item_size;
                let diff_count = diff_count.floor();
                self.anchor_index =
                    (self.anchor_index + diff_count as i64).clamp(self.first_item, self.last_item);
                self.scroll_offset_from_anchor -= diff_count * mean_item_size;
            } else {
                debug_assert_eq!(
                    count, 0,
                    "Assumption: If the division produced an infinite result, it's because of a divide by zero."
                );
                // TODO: How can we handle this sanely? We're scrolled very far down (3 pages + 3000 logical pixels),
                // but we don't have any idea how big the items are; that could be within a single item.
                // I guess we need to just wait for the anchor to be loaded?
            }
        } else {
            let previous_anchor = self.anchor_index;
            loop {
                if self.scroll_offset_from_anchor < 0. {
                    if self.anchor_index == self.first_item {
                        self.scroll_offset_from_anchor = 0.;
                        break;
                    }
                    let new_idx = self.anchor_index - 1;
                    let new_anchor = self.current_items.get(&new_idx);
                    let new_anchor_height = if let Some(new_anchor) = new_anchor {
                        ctx.child_size(new_anchor).height
                    } else {
                        // We need to request this new anchor to be loaded
                        break;
                    };
                    self.scroll_offset_from_anchor += new_anchor_height;
                    height_before_anchor -= new_anchor_height;
                    height_after_anchor += new_anchor_height;
                    self.anchor_index = new_idx;
                } else {
                    let anchor_pod = self.current_items.get(&self.anchor_index);
                    let anchor_height = if let Some(anchor_pod) = anchor_pod {
                        ctx.child_size(anchor_pod).height
                    } else {
                        // We need to request this new anchor to be loaded
                        break;
                    };
                    if self.scroll_offset_from_anchor > anchor_height {
                        if self.anchor_index != self.last_item {
                            self.scroll_offset_from_anchor = anchor_height;
                            break;
                        }
                        self.scroll_offset_from_anchor -= anchor_height;
                        height_before_anchor += anchor_height;
                        self.anchor_index += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        // TODO: I *suspect* we need to do something else?
        // self.anchor_index = new_anchor;

        // TODO: How can we even plausibly handle arbitrary transforms?
        let mut y = -height_before_anchor;
        for (idx, item) in &mut self.current_items {
            let size = ctx.child_size(item);
            ctx.place_child(item, Point::new(0., y));
            // TODO: Padding/gap?
            y += size.height;
        }

        let target_range = if mean_item_size.is_finite() {
            let up = (viewport_size.height * 1.5 / mean_item_size).ceil() as i64 + 1;
            let down = (viewport_size.height * 2.5 / mean_item_size).ceil() as i64 + 1;
            (self.anchor_index - up)..(self.anchor_index + down)
        } else {
            ((self.anchor_index - 2)..(self.anchor_index + 5))
        };
        let mut additions = SmallVec::new();
        let mut removals = SmallVec::new();
        for id in target_range.clone() {
            if !self.current_items.contains_key(&id) {
                additions.push(id);
            }
        }
        for id in self.current_items.keys() {
            if !target_range.contains(id) {
                removals.push(*id);
            }
        }
        ctx.submit_action(crate::core::Action::Other(Box::new(VirtualScrollAction {
            add: additions,
            remove: removals,
        })));
        viewport_size
    }

    fn compose(&mut self, ctx: &mut crate::core::ComposeCtx) {
        let translation = Vec2 {
            x: 0.,
            y: self.scroll_offset_from_anchor,
        };
        for child in self.current_items.values_mut() {
            ctx.set_child_scroll_translation(child, translation);
        }
    }

    fn accessibility_role(&self) -> accesskit::Role {
        // TODO: accesskit::Role::ScrollView ?
        accesskit::Role::GenericContainer
    }

    fn accessibility(&mut self, ctx: &mut crate::core::AccessCtx, node: &mut accesskit::Node) {
        // TODO: Better virtual scrolling accessibility
        // Intended as a follow-up collaboration with Matt
        node.set_clips_children();
    }

    fn on_access_event(
        &mut self,
        ctx: &mut crate::core::EventCtx,
        event: &crate::core::AccessEvent,
    ) {
        // TODO: Handle scroll events
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut crate::core::EventCtx,
        event: &crate::core::PointerEvent,
    ) {
        // TODO: Handle scroll wheel
    }
    fn accepts_pointer_interaction(&self) -> bool {
        // We handle e.g. scroll wheel events
        true
    }

    fn children_ids(&self) -> smallvec::SmallVec<[crate::core::WidgetId; 16]> {
        self.current_items.iter().map(|(_, pod)| pod.id()).collect()
    }

    fn register_children(&mut self, ctx: &mut crate::core::RegisterCtx) {
        for (_, child) in &mut self.current_items {
            ctx.register_child(child);
        }
    }

    fn paint(&mut self, _ctx: &mut crate::core::PaintCtx, _scene: &mut vello::Scene) {}

    fn on_text_event(&mut self, ctx: &mut crate::core::EventCtx, event: &crate::core::TextEvent) {
        // Maybe? Handle pagedown? or something like escape for keyboard focus to escape the virtual list
    }
    fn accepts_text_input(&self) -> bool {
        false
    }

    fn update(&mut self, ctx: &mut crate::core::UpdateCtx, event: &crate::core::Update) {
        match event {
            crate::core::Update::WidgetAdded => {}
            crate::core::Update::DisabledChanged(_) => {}
            crate::core::Update::StashedChanged(_) => {}
            crate::core::Update::RequestPanToChild(rect) => {} // TODO,
            crate::core::Update::HoveredChanged(_) => {}
            crate::core::Update::ChildHoveredChanged(_) => {}
            crate::core::Update::FocusChanged(_) => {
                if cfg!(debug_assertions) {
                    unreachable!()
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

    // TODO: Optimise?
    // fn find_widget_at_pos<'c>(
    //     &'c self,
    //     ctx: crate::core::QueryCtx<'c>,
    //     pos: vello::kurbo::Point,
    // ) -> Option<crate::core::WidgetRef<'c, dyn Widget>> {
    // }
}
