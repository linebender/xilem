// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! An example of a blocking function running in another thread. We give
//! the other thread some data and then we also pass some data back
//! to the main thread using commands.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use std::{thread, time};

use masonry::widget::prelude::*;
use masonry::widget::WidgetRef;
use masonry::widget::{Flex, Label, Spinner, WidgetPod};
use masonry::{AppLauncher, Point, Selector, Target, WindowDescription};
use smallvec::{smallvec, SmallVec};

const FINISH_SLOW_FUNCTION: Selector<u32> = Selector::new("finish_slow_function");

struct MainWidget {
    pub content: WidgetPod<Flex>,
    pub loading: bool,
    pub value: u32,
}

impl MainWidget {
    fn new(value: u32) -> Self {
        MainWidget {
            content: WidgetPod::new(
                Flex::column().with_child(Label::new("Click to change value: 0")),
            ),
            loading: false,
            value,
        }
    }
}

impl Widget for MainWidget {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event) {
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    ctx.set_active(false);
                    if !self.loading {
                        self.loading = true;

                        let number = self.value + 1;
                        ctx.run_in_background(move |event_sink| {
                            // "sleep" stands in for a long computation, a download, etc.
                            thread::sleep(time::Duration::from_millis(2000));

                            event_sink
                                .submit_command(FINISH_SLOW_FUNCTION, number, Target::Auto)
                                .expect("command failed to submit");
                        });

                        self.content.on_event(ctx, event);
                        let mut flex_mut = ctx.get_mut(&mut self.content);
                        flex_mut.clear();
                        flex_mut.add_child(Spinner::new());

                        return;
                    }
                }
            }
            Event::Command(cmd) if cmd.is(FINISH_SLOW_FUNCTION) => {
                let value = *cmd.get(FINISH_SLOW_FUNCTION);

                self.content.on_event(ctx, event);

                let mut flex_mut = ctx.get_mut(&mut self.content);
                flex_mut.clear();
                flex_mut.add_child(Label::new(format!("New value: {value}")));

                self.loading = false;
                self.value = value;

                return;
            }
            _ => (),
        }
        self.content.on_event(ctx, event);
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.content.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let content_size = self.content.layout(ctx, bc);
        ctx.place_child(&mut self.content, Point::ORIGIN);
        content_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        self.content.paint(ctx);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        smallvec![self.content.as_dyn()]
    }
}

// ---

fn main() {
    let main_window = WindowDescription::new(MainWidget::new(0)).title("Blocking functions");
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch()
        .expect("launch failed");
}
