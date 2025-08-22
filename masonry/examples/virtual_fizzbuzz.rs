// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A demonstration of the [`VirtualScroll`] widget, producing an infinite[^1] FizzBuzz.
//!
//! [^1]: Limited to `i64::MIN..i64::MAX-1`; that is, there are `2^64-1` possible items.
//! However, there is (currently...) no way to jump to a specific item, so it's impossible to reach the end.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::core::{ArcStr, ErasedAction, NewWidget, StyleProperty, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::theme::default_property_set;
use masonry::widgets::{Label, VirtualScroll, VirtualScrollAction};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};

/// Function to create the virtual scroll area.
fn init() -> VirtualScroll {
    // We start our fizzbuzzing with the top of the screen at item 0
    VirtualScroll::new(0)
}

struct Driver {
    scroll_id: WidgetId,
    fizz: ArcStr,
    buzz: ArcStr,
    fizzbuzz: ArcStr,
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if widget_id == self.scroll_id {
            // The VirtualScroll widget will send us a VirtualScrollAction every time it wants different
            // items to be loaded or unloaded.
            let action = action
                .downcast::<VirtualScrollAction>()
                .expect("Only expected Virtual Scroll actions");
            ctx.render_root(window_id).edit_root_widget(|mut root| {
                let mut scroll = root.downcast::<VirtualScroll>();
                // We need to tell the `VirtualScroll` which request this is associated with
                // This is so that the controller knows which actions have been handled.
                VirtualScroll::will_handle_action(&mut scroll, &action);
                for idx in action.old_active.clone() {
                    if !action.target.contains(&idx) {
                        // If we had different work to do in response to the item being unloaded
                        // (for example, saving some related data?), then we'd do it here
                        VirtualScroll::remove_child(&mut scroll, idx);
                    }
                }
                for idx in action.target.clone() {
                    if !action.old_active.contains(&idx) {
                        let label: ArcStr = match (idx % 3 == 0, idx % 5 == 0) {
                            (false, true) => self.buzz.clone(),
                            (true, false) => self.fizz.clone(),
                            (true, true) => self.fizzbuzz.clone(),
                            (false, false) => format!("{idx}").into(),
                        };
                        VirtualScroll::add_child(
                            &mut scroll,
                            idx,
                            NewWidget::new(Label::new(label).with_style(StyleProperty::FontSize(
                                if idx % 100 == 0 { 40. } else { 20. },
                            )))
                            .erased(),
                        );
                    }
                }
            });
        } else {
            tracing::warn!("Got unexpected action {action:?}");
        }
    }
}

fn main() {
    let scroll_id = WidgetId::next();
    let main_widget = NewWidget::new_with_id(init(), scroll_id).erased();
    let driver = Driver {
        scroll_id,
        fizz: "Fizz".into(),
        buzz: "Buzz".into(),
        fizzbuzz: "FizzBuzz".into(),
        window_id: WindowId::next(),
    };
    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("Infinite FizzBuzz")
        .with_resizable(true)
        .with_min_surface_size(window_size);

    let (event_sender, event_receiver) =
        std::sync::mpsc::channel::<masonry_winit::app::MasonryUserEvent>();

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::builder(),
        event_sender,
        event_receiver,
        vec![NewWindow::new_with_id(
            driver.window_id,
            window_attributes,
            main_widget,
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
