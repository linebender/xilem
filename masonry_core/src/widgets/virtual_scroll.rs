// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused, reason = "Development")]
use std::collections::{BTreeMap, VecDeque};

use vello::kurbo::Vec2;

use crate::core::{Widget, WidgetPod};

pub struct VirtualScrollAction;

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
    current_items: VecDeque<(i64, WidgetPod<W>)>,
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
    approx_item_height: f64,
}

impl<W: Widget> Widget for VirtualScroll<W> {
    fn layout(
        &mut self,
        ctx: &mut crate::core::LayoutCtx,
        bc: &crate::core::BoxConstraints,
    ) -> vello::kurbo::Size {
        // Oh, OK, we actually already know the scroll_offset_from_anchor here...
        let size = bc.max();
        // What do we need the total size above the anchor to be?
        ctx.set_clip_path(size.to_rect());
        size
        // TODO: How can we even plausibly handle arbitrary transforms?
    }

    fn compose(&mut self, ctx: &mut crate::core::ComposeCtx) {
        let translation = Vec2 {
            x: 0.,
            y: self.scroll_offset_from_anchor,
        };
        for (_, child) in &mut self.current_items {
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
