use std::{
    collections::BinaryHeap,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use crate::WidgetId;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TimerId(NonZeroU64);

impl TimerId {
    pub fn next() -> TimerId {
        static TIMER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        TimerId(id.try_into().unwrap())
    }
}
/// An ordered list of timers set by masonry
///
/// Implemented as a min priority queue
pub struct TimerQueue {
    queue: BinaryHeap<Timer>,
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    pub fn push(&mut self, timer: Timer) {
        self.queue.push(timer);
    }

    /// Copy and return the `Instant` at the head of the queue
    pub fn peek(&self) -> Option<Timer> {
        self.queue.peek().map(|v| *v)
    }

    /// Remove the `Instant` at the head of the queue and return it
    pub fn pop(&mut self) -> Option<Timer> {
        self.queue.pop()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Timer {
    pub id: TimerId,
    pub widget_id: WidgetId,
    pub deadline: Instant,
}

impl Timer {
    pub fn new(widget_id: WidgetId, deadline: Instant) -> Self {
        Self {
            id: TimerId::next(),
            widget_id,
            deadline,
        }
    }
}

// We implement `Ord` first by comparing `deadline`, and then
// `id`. This way, we ensure that timers with the same expiry
// time will trigger in the order they were created.
//
// Because Rust std's `BinaryHeap` is max-first, we need to reverse
// both comparisons.
impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deadline
            .cmp(&other.deadline)
            .reverse()
            .then(self.id.cmp(&other.id).reverse())
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
