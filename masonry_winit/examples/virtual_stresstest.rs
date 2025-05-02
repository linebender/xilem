// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A demonstration of the [`VirtualScroll`] widget, producing an automatically scrolling list of inputs.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app::{AppDriver, DriverCtx, EventLoop, EventLoopBuilder};
use masonry::core::{Action, StyleProperty, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::widgets::{Label, RootWidget, VirtualScroll, VirtualScrollAction};

use winit::error::EventLoopError;
use winit::window::Window;

/// The widget kind contained in the scroll area. This is a type parameter (`W`) of [`VirtualScroll`],
/// although note that [`dyn Widget`](masonry::core::Widget) can also be used for dynamic children kinds.
///
/// We use a type alias for this, as when we downcast to the `VirtualScroll`, we need to be sure to
/// always use the same type for `W`.
type ScrollContents = Label;

/// Function to create the virtual scroll area.
fn init() -> VirtualScroll<ScrollContents> {
    // We start our scrolling with the top of the screen at item 0
    VirtualScroll::new(0)
        .with_valid_range(0..i64::MAX)
        .with_scroll_per_frame(Some(600.))
}

const FONT_SIZE: f32 = 8_f32;

struct Driver {
    scroll_id: WidgetId,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action) {
        if widget_id == self.scroll_id {
            if let Action::Other(action) = action {
                // The VirtualScroll widget will send us a VirtualScrollAction every time it wants different
                // items to be loaded or unloaded.
                let action = action.downcast::<VirtualScrollAction>().unwrap();
                ctx.render_root().edit_root_widget(|mut root| {
                    let mut root = root.downcast::<RootWidget<VirtualScroll<ScrollContents>>>();
                    let mut scroll = RootWidget::child_mut(&mut root);
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
                            let label = calc_label(idx);

                            VirtualScroll::add_child(
                                &mut scroll,
                                idx,
                                WidgetPod::new(
                                    Label::new(label)
                                        .with_style(StyleProperty::FontSize(FONT_SIZE))
                                        .with_style(StyleProperty::LineHeight(1.0)),
                                ),
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

// Works around rustfmt failing if this is inlined.
fn calc_label(idx: i64) -> String {
    format!("{idx}: Long lines with a varying font weight should be a worst case for scrolling.")
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let main_widget = WidgetPod::new(init());
    let driver = Driver {
        scroll_id: main_widget.id(),
    };
    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Infinite FizzBuzz")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::app::run(
        event_loop,
        window_attributes,
        RootWidget::from_pod(main_widget),
        driver,
    )
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
