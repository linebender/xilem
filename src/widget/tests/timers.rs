// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::cell::Cell;
use std::rc::Rc;

use instant::Duration;

use crate::testing::{ModularWidget, TestHarness};
use crate::*;

#[test]
fn basic_timer() {
    let timer_handled: Rc<Cell<bool>> = Rc::new(false.into());

    let widget = ModularWidget::new((None, timer_handled.clone()))
        .lifecycle_fn(move |state, ctx, event| match event {
            LifeCycle::WidgetAdded => {
                state.0 = Some(ctx.request_timer(Duration::from_secs(3)));
            }
            _ => {}
        })
        .event_fn(|state, _ctx, event| {
            if let Event::Timer(token) = event {
                if *token == state.0.unwrap() {
                    state.1.set(true);
                }
            }
        });

    let mut harness = TestHarness::create(widget);

    assert_eq!(timer_handled.get(), false);

    harness.move_timers_forward(Duration::from_secs(1));
    assert_eq!(timer_handled.get(), false);

    harness.move_timers_forward(Duration::from_secs(2));
    assert_eq!(timer_handled.get(), true);
}
