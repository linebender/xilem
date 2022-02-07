// Copyright 2019 The Druid Authors.

//! An example of a blocking function running in another thread. We give
//! the other thread some data and then we also pass some data back
//! to the main thread using commands.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use smallvec::{smallvec, SmallVec};
use std::{thread, time};

use widget_cruncher::widget::prelude::*;
use widget_cruncher::widget::{Flex, Label, Spinner, WidgetPod};
use widget_cruncher::{AppLauncher, Point, Selector, Target, WindowDesc};

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
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        ctx.init();
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

                        self.content.on_event(ctx, event, env);
                        let mut flex_view = ctx.get_child_view(&mut self.content);
                        flex_view.clear();
                        flex_view.add_child(Spinner::new());

                        return;
                    }
                }
            }
            Event::Command(cmd) if cmd.is(FINISH_SLOW_FUNCTION) => {
                let value = *cmd.get(FINISH_SLOW_FUNCTION);

                self.content.on_event(ctx, event, env);

                let mut flex_view = ctx.get_child_view(&mut self.content);
                flex_view.clear();
                flex_view.add_child(Label::new(format!("New value: {}", value)));

                self.loading = false;
                self.value = value;

                return;
            }
            _ => (),
        }
        self.content.on_event(ctx, event, env);
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.content.lifecycle(ctx, event, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        let content_size = self.content.layout(ctx, bc, env);
        self.content.set_origin(ctx, env, Point::ORIGIN);
        content_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        self.content.paint(ctx, env);
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        smallvec![&self.content as &dyn AsWidgetPod]
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        smallvec![&mut self.content as &mut dyn AsWidgetPod]
    }
}

// ---

fn main() {
    let main_window = WindowDesc::new(MainWidget::new(0)).title("Blocking functions");
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch()
        .expect("launch failed");
}
