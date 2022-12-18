// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::collections::VecDeque;

use druid_shell::TimerToken;
use instant::Duration;

/// Handles timers for unit tests.
///
/// In normal app execution, timers are submitted to the platform handle, which immediately
/// returns a token. The token is stored in a HashMap with a WidgetId and, when the timer
/// fires, the platform passes us the token again so we can plumb the event to the right
/// widget.
///
/// In unit tests, we can't submit timers to the platform. Instead, we store a list of
/// timer tokens and durations, and when the user calls [`TestHarness::move_timers_forward`],
/// the timers are "manually" mutated and checked, and the matching events fired.
///
/// To avoid polluting the code with `#[cfg(test)]` annotations, MockTimerQueue is also
/// present in non-test code, but it's always empty.
pub(crate) struct MockTimerQueue {
    pub current_time: Duration,
    pub queue: VecDeque<(Duration, TimerToken)>,
}

impl MockTimerQueue {
    pub(crate) fn new() -> Self {
        MockTimerQueue {
            current_time: Duration::ZERO,
            queue: VecDeque::new(),
        }
    }

    #[must_use]
    pub(crate) fn add_timer(&mut self, duration: Duration) -> TimerToken {
        let deadline = self.current_time + duration;
        let token = TimerToken::next();
        let idx = self
            .queue
            .binary_search_by_key(&deadline, |(d, _t)| *d)
            .unwrap_or_else(|x| x);
        self.queue.insert(idx, (deadline, token));

        token
    }

    #[must_use]
    pub(crate) fn move_forward(&mut self, duration: Duration) -> Vec<TimerToken> {
        self.current_time += duration;
        let idx = self
            .queue
            .partition_point(|(deadline, _token)| *deadline <= self.current_time);

        self.queue
            .drain(0..idx)
            .map(|(_deadline, token)| token)
            .collect()
    }
}
