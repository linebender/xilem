// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows driving a Xilem application from a pre-existing Winit event loop.
//! Currently, this supports running as its own window alongside an existing application, or
//! accessing raw events from winit.
//! Support for more custom embeddings would be welcome, but needs more design work

use masonry::properties::types::AsUnit;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::theme::default_property_set;
use masonry_winit::app::{AppDriver, MasonryUserEvent};
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};
use xilem::view::{Label, button, flex_row, label, sized_box};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

/// A component to make a bigger than usual button.
fn big_button<F: Fn(&mut i32) + Send + Sync + 'static>(
    label: impl Into<Label>,
    callback: F,
) -> impl WidgetView<Edit<i32>> {
    // This being fully specified is "a known limitation of the trait solver"
    sized_box(button::<Edit<i32>, _, _, F>(label.into(), callback))
        .width(40.px())
        .height(40.px())
}

fn app_logic(data: &mut i32) -> impl WidgetView<Edit<i32>> + use<> {
    flex_row((
        big_button("-", |data| {
            *data -= 1;
        }),
        label(format!("count: {data}")).text_size(32.),
        big_button("+", |data| {
            *data += 1;
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

/// An application not managed by Xilem, but which wishes to embed Xilem.
struct ExternalApp {
    masonry_state: masonry_winit::app::MasonryState<'static>,
    app_driver: Box<dyn AppDriver>,
}

impl ApplicationHandler<MasonryUserEvent> for ExternalApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state
            .handle_resumed(event_loop, &mut *self.app_driver);
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_suspended(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_about_to_wait(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.masonry_state.handle_window_event(
            event_loop,
            window_id,
            event,
            self.app_driver.as_mut(),
        );
    }

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: MasonryUserEvent,
    ) {
        self.masonry_state
            .handle_user_event(event_loop, event, self.app_driver.as_mut());
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        // Handle the escape key to exit the app outside of masonry/xilem
        if let winit::event::DeviceEvent::Key(key) = &event
            && key.state == ElementState::Pressed
            && key.physical_key == PhysicalKey::Code(KeyCode::Escape)
        {
            event_loop.exit();
            return;
        }

        self.masonry_state.handle_device_event(
            event_loop,
            device_id,
            event,
            self.app_driver.as_mut(),
        );
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.masonry_state.handle_new_events(event_loop, cause);
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_memory_warning(event_loop);
    }
}

fn main() -> Result<(), EventLoopError> {
    let window_size = winit::dpi::LogicalSize::new(800.0, 800.0);
    let window_options = WindowOptions::new("External event loop").with_min_inner_size(window_size);

    let xilem = Xilem::new_simple(0, app_logic, window_options);

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let (driver, windows) =
        xilem.into_driver_and_windows(move |event| proxy.send_event(event).map_err(|err| err.0));
    let masonry_state = masonry_winit::app::MasonryState::new(
        event_loop.create_proxy(),
        windows,
        default_property_set(),
    );

    let mut app = ExternalApp {
        masonry_state,
        app_driver: Box::new(driver),
    };
    event_loop.run_app(&mut app)
}
