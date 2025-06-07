// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A demonstration of the [`VirtualScroll`] widget, producing an infinite[^1] FizzBuzz.
//!
//! [^1]: Limited to `i64::MIN..i64::MAX-1`; that is, there are `2^64-1` possible items.
//! However, there is (currently...) no way to jump to a specific item, so it's impossible to reach the end.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use masonry_winit::core::{Action, ArcStr, StyleProperty, WidgetId, WidgetPod};
use masonry_winit::dpi::LogicalSize;
use masonry_winit::widgets::{Label, RootWidget, VirtualScroll, VirtualScrollAction};
use winit::window::Window;

/// The widget kind contained in the scroll area. This is a type parameter (`W`) of [`VirtualScroll`],
/// although note that [`dyn Widget`](masonry_winit::core::Widget) can also be used for dynamic children kinds.
///
/// We use a type alias for this, as when we downcast to the `VirtualScroll`, we need to be sure to
/// always use the same type for `W`.
type ScrollContents = Label;

/// Function to create the virtual scroll area.
fn init() -> VirtualScroll<ScrollContents> {
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
        action: Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if widget_id == self.scroll_id {
            if let Action::Other(action) = action {
                // The VirtualScroll widget will send us a VirtualScrollAction every time it wants different
                // items to be loaded or unloaded.
                let action = action.downcast::<VirtualScrollAction>().unwrap();
                ctx.render_root(window_id).edit_root_widget(|mut root| {
                    let mut root = root.downcast::<RootWidget>();
                    let mut scroll = RootWidget::child_mut(&mut root);
                    let mut scroll = scroll.downcast::<VirtualScroll<ScrollContents>>();
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
                                WidgetPod::new(Label::new(label).with_style(
                                    StyleProperty::FontSize(if idx % 100 == 0 { 40. } else { 20. }),
                                )),
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

fn main() {
    let main_widget = WidgetPod::new(init()).erased();
    let driver = Driver {
        scroll_id: main_widget.id(),
        fizz: "Fizz".into(),
        buzz: "Buzz".into(),
        fizzbuzz: "FizzBuzz".into(),
        window_id: WindowId::next(),
    };
    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Infinite FizzBuzz")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![(
            driver.window_id,
            window_attributes,
            Box::new(RootWidget::from_pod(main_widget)),
        )],
        driver,
    )
    .unwrap();
}
