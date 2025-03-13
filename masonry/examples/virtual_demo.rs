// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows how to use a grid layout in Masonry.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app::{AppDriver, DriverCtx};
use masonry::core::{Action, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::widgets::{Label, RootWidget, VirtualScroll, VirtualScrollAction};

use winit::window::Window;

struct Driver {
    scroll_id: WidgetId,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action) {
        if widget_id == self.scroll_id {
            if let Action::Other(action) = action {
                let action = action.downcast::<VirtualScrollAction>().unwrap();
                ctx.render_root().edit_root_widget(|mut root| {
                    let mut root = root.downcast::<RootWidget<VirtualScroll<ScrollContents>>>();
                    let mut scroll = RootWidget::child_mut(&mut root);
                    for idx in action.old_active.clone() {
                        if !action.target.contains(&idx) {
                            eprintln!("Removing {idx}");
                            VirtualScroll::remove_child(&mut scroll, idx);
                        }
                    }
                    for idx in action.target.clone() {
                        if !action.old_active.contains(&idx) {
                            eprintln!("Adding {idx}");
                            VirtualScroll::add_child(
                                &mut scroll,
                                idx,
                                WidgetPod::new(Label::new(format!("Child {idx}"))),
                            );
                        }
                    }
                });
            }
        } else {
            tracing::warn!("Got unexpected action {action:?}");
        }
    }
}

type ScrollContents = Label;

fn make_scroll() -> VirtualScroll<ScrollContents> {
    VirtualScroll::new(0)
}

fn main() {
    let main_widget = WidgetPod::new(make_scroll());
    let driver = Driver {
        scroll_id: main_widget.id(),
    };
    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Grid Layout")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::app::run(
        masonry::app::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::from_pod(main_widget),
        driver,
    )
    .unwrap();
}
