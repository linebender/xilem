use std::collections::VecDeque;
use std::panic::Location;
use std::path::Path;
use std::sync::Arc;

use crate::command::CommandQueue;
use crate::debug_logger::DebugLogger;
use crate::ext_event::ExtEventQueue;
use crate::piet::{BitmapTarget, Device, Error, ImageFormat, Piet};
use crate::platform::PendingWindow;
use crate::widget::widget_view::WidgetRef;
use crate::widget::WidgetState;
use crate::*;
use druid_shell::{KeyEvent, Modifiers, MouseButton, MouseButtons};
pub use druid_shell::{
    RawMods, Region, Scalable, Scale, Screen, SysMods, TimerToken, WindowHandle, WindowLevel,
    WindowState,
};
use instant::Duration;

// TODO - Document
// - Explain why mock timers are useful
// - Explain corner cases (eg one event + layout per timer even when timers are simultaneous)
// - Refer to this doc everywhere else in the code that uses mock timers
// TODO - remove pub
pub struct MockTimerQueue {
    pub current_time: Duration,
    pub queue: VecDeque<(Duration, TimerToken)>,
}

impl MockTimerQueue {
    pub fn new() -> Self {
        MockTimerQueue {
            current_time: Duration::ZERO,
            queue: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn add_timer(&mut self, duration: Duration) -> TimerToken {
        let deadline = self.current_time + duration;
        let token = TimerToken::next();
        let idx = self
            .queue
            .binary_search_by_key(&deadline, |(d, t)| *d)
            .unwrap_or_else(|x| x);
        self.queue.insert(idx, (deadline, token));

        token
    }

    #[must_use]
    pub fn move_forward(&mut self, duration: Duration) -> Vec<TimerToken> {
        self.current_time += duration;
        let idx = self
            .queue
            .partition_point(|(deadline, token)| *deadline <= self.current_time);

        self.queue
            .drain(0..idx)
            .map(|(deadline, token)| token)
            .collect()
    }
}
