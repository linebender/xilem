// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows driving a Xilem application from a pre-existing Winit event loop.
//! Currently, this supports running as its own window alongside an existing application, or
//! accessing raw events from winit.
//! Support for more custom embeddings would be welcome, but needs more design work
use masonry::{
    app_driver::AppDriver,
    widget::{CrossAxisAlignment, MainAxisAlignment},
    ArcStr,
};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};
use xilem::{
    view::{button, flex, label, sized_box},
    EventLoop, WidgetView, Xilem,
};

/// A component to make a bigger than usual button
fn big_button(
    label: impl Into<ArcStr>,
    callback: impl Fn(&mut i32) + Send + Sync + 'static,
) -> impl WidgetView<i32> {
    sized_box(button(label, callback)).width(40.).height(40.)
}

fn app_logic(data: &mut i32) -> impl WidgetView<i32> {
    flex((
        big_button("-", |data| {
            *data -= 1;
        }),
        label(format!("count: {}", data)).text_size(32.),
        big_button("+", |data| {
            *data += 1;
        }),
    ))
    .direction(xilem::Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

/// An application not managed by Xilem, but which wishes to embed Xilem.
struct ExternalApp {
    masonry_state: masonry::event_loop_runner::MasonryState<'static>,
    app_driver: Box<dyn AppDriver>,
}

impl ApplicationHandler<accesskit_winit::Event> for ExternalApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.masonry_state.handle_resumed(event_loop);
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
        event: accesskit_winit::Event,
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
        if let winit::event::DeviceEvent::Key(key) = &event {
            if key.state == ElementState::Pressed
                && key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            {
                event_loop.exit();
                return;
            }
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
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("External event loop".to_string())
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let xilem = Xilem::new(0, app_logic);

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let masonry_state = masonry::event_loop_runner::MasonryState::new(
        window_attributes,
        &event_loop,
        xilem.root_widget,
    );

    let mut app = ExternalApp {
        masonry_state,
        app_driver: Box::new(xilem.driver),
    };
    event_loop.run_app(&mut app)
}
