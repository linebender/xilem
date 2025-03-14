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
    // We know this means that the final item can't be included. That's pretty unavoidable.
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
    /// Used to determine if scrolling will require a relayout (because the anchor will have changed).
    anchor_height: f64,

    /// The available width in the last layout call, used so that layout of children can be skipped if it won't have changed.
    old_width: f64,

    layouts_since_compose: u64,
    warned_not_dense: bool,
}

impl<W: Widget + ?Sized> VirtualScroll<W> {
    pub fn new(initial_anchor: i64) -> Self {
        Self {
            valid_range: i64::MIN..i64::MAX,
            // Both of these ranges start empty; this is intentional
            active_range: initial_anchor..initial_anchor,
            target_range: initial_anchor..initial_anchor,
            items: HashMap::default(),
            anchor_index: initial_anchor,
            scroll_offset_from_anchor: 0.0,
            mean_item_height: DEFAULT_MEAN_ITEM_HEIGHT,
            anchor_height: DEFAULT_MEAN_ITEM_HEIGHT,
            old_width: f64::NAN,
            layouts_since_compose: 0,
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
}

impl<W: Widget + ?Sized> Widget for VirtualScroll<W> {
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
        // We lay all of the active items out, even if some of them will be stashed imminently.
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

        let target_range = if mean_item_height.is_finite() {
            let start = if let Some(item_crossing_top) = item_crossing_top {
                item_crossing_top
            } else {
                let number_needed =
                    ((cutoff_up - height_before_anchor) / mean_item_height).ceil() as i64;
                self.active_range.start - number_needed
            };
            let end = if y > viewport_size.height * 2.5 {
                item_crossing_bottom + 1
            } else {
                // `y` is the bottom of the bottommost loaded item
                let number_needed = ((cutoff_down - y) / mean_item_height).ceil() as i64;
                self.active_range.end + number_needed
            };
            start..end
        } else {
            ((self.anchor_index - 2)..(self.anchor_index + 5))
        };
        // Avoid requesting invalid items
        let target_range = target_range.start.max(self.valid_range.start)
            ..target_range.end.min(self.valid_range.end);
        self.target_range = target_range;

        if self.target_range != self.active_range {
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
                // TODO: Something about this is invalid, and it's not clear what
                // for idx in this.widget.target_range.clone() {
                //     if !this.widget.active_range.contains(&idx) {
                //         let item = this.widget.items.get_mut(&idx);
                //         if let Some(item) = item {
                //             // TODO: Are there reasonable cases where this happens?
                //             // A widget that scrolls a different widget into view in response to being stashed?!
                //             tracing::debug!(
                //                 "Found item {:?} (index {idx}) which should have been removed by driver, in `VirtualScroll`", item.id()
                //             );
                //             this.ctx.set_stashed(item, false);
                //         }
                //     }
                // }
                this.widget.active_range = this.widget.target_range.clone();
            });
            ctx.submit_action(crate::core::Action::Other(Box::new(VirtualScrollAction {
                old_active: self.active_range.clone(),
                target: self.target_range.clone(),
            })));
        }
        // TODO: We should still try and find a way to detect infinite loops;
        // Ideally, actions would be handled during the mutate pass, but that isn't how things are set up.

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
                if self.scroll_offset_from_anchor < 0.
                    || self.scroll_offset_from_anchor > self.anchor_height
                {
                    ctx.request_layout();
                }
                ctx.request_compose();
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
                    let delta = 2000.;
                    if matches!(key_event.logical_key, Key::Named(NamedKey::PageDown)) {
                        self.scroll_offset_from_anchor += delta;
                        if self.scroll_offset_from_anchor < 0.
                            || self.scroll_offset_from_anchor > self.anchor_height
                        {
                            ctx.request_layout();
                        }
                        ctx.request_compose();
                    }
                    if matches!(key_event.logical_key, Key::Named(NamedKey::PageUp)) {
                        self.scroll_offset_from_anchor -= delta;
                        if self.scroll_offset_from_anchor < 0.
                            || self.scroll_offset_from_anchor > self.anchor_height
                        {
                            ctx.request_layout();
                        }
                        ctx.request_compose();
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
        true
    }

    // TODO: Optimise using binary search?
    // fn find_widget_at_pos(..);
}
