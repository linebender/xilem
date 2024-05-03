// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A virtualized list of items.
//!
//! Note that this for experimentation and may be hacky in a number of
//! ways not ideal for production use.

use std::collections::BTreeMap;

use druid_shell::kurbo::{Point, Rect, Size};

use crate::{event::Event, id::IdPath, Widget};

use super::{
    EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, PreparePaintCx, RawEvent, UpdateCx,
};

pub struct List {
    id_path: IdPath,
    n_items: usize,
    item_height: f64,
    items: BTreeMap<usize, Pod>,
    item_range: (usize, usize),
}

/// A request to change children.
///
/// This is an event sent by the list widget to the client when the viewport changes.
/// Correct handling is for each index in `add` to result in a `set_child` call, and
/// corresponding for `remove` and `remove_child`.
#[derive(Debug)]
pub struct ListChildRequest {
    pub add: Vec<usize>,
    pub remove: Vec<usize>,
}

impl List {
    pub fn new(id_path: IdPath, n_items: usize, item_height: f64) -> Self {
        List {
            id_path,
            n_items,
            item_height,
            items: BTreeMap::new(),
            item_range: (0, 0),
        }
    }

    pub fn set_child(&mut self, i: usize, child: Pod) {
        self.items.insert(i, child);
    }

    pub fn remove_child(&mut self, i: usize) {
        self.items.remove(&i);
    }

    /// Note: this will panic if the child is not set. The client is
    /// responsible for tracking which children are set.
    pub fn child_mut(&mut self, i: usize) -> &mut Pod {
        self.items.get_mut(&i).unwrap()
    }
}

impl Widget for List {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        for (_, child) in &mut self.items {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        for (_, child) in &mut self.items {
            child.lifecycle(cx, event);
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        for (_, child) in &mut self.items {
            child.update(cx);
        }
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        for (_, child) in &mut self.items {
            child.measure(cx);
        }
        let height = self.n_items as f64 * self.item_height;
        (Size::new(0.0, height), Size::new(1e9, height))
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        // TODO: recurse to children
        let child_proposed = Size::new(proposed_size.width, self.item_height);
        for (i, child) in &mut self.items {
            let _ = child.layout(cx, child_proposed);
            child.state.origin = Point::new(0.0, *i as f64 * self.item_height);
        }
        let height = self.n_items as f64 * self.item_height;
        Size::new(proposed_size.width, height)
    }

    fn prepare_paint(&mut self, cx: &mut PreparePaintCx, visible: Rect) {
        let start = ((visible.y0 / self.item_height).floor() as usize).saturating_sub(10).min(self.n_items);
        let end = ((visible.y1 / self.item_height).ceil() as usize + 10).min(self.n_items);
        if (start, end) != self.item_range {
            // item range has changed, send a request
            let mut add = Vec::new();
            let mut remove = Vec::new();
            if self.item_range.1 <= start || self.item_range.0 >= end {
                add.extend(start..end);
                remove.extend(self.item_range.0..self.item_range.1);
            } else {
                if self.item_range.0 < start {
                    remove.extend(self.item_range.0..start);
                } else if start < self.item_range.0 {
                    add.extend((start..self.item_range.0).rev());
                }
                if self.item_range.1 < end {
                    add.extend(self.item_range.1..end);
                } else if end < self.item_range.1 {
                    remove.extend(end..self.item_range.1);
                }
            }
            let req = ListChildRequest { add, remove };
            //println!("req: {:?}", req);
            cx.add_event(Event::new(self.id_path.clone(), req));
            self.item_range = (start, end);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        for (_, child) in &mut self.items {
            child.paint(cx);
        }
    }
}
