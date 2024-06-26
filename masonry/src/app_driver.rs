// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::event_loop_runner::MasonryState;
use crate::widget::WidgetMut;
use crate::{Action, Widget, WidgetId};

// xilem::App will implement AppDriver

pub struct DriverCtx<'a> {
    // TODO
    // This is exposed publicly for now to let people drive
    // masonry on their own, but this is not expected to be
    // stable or even supported. This is for short term
    // expedience only while better solutions are devised.
    #[doc(hidden)]
    pub main_root_widget: WidgetMut<'a, Box<dyn Widget>>,
}

#[allow(unused_variables)]
pub trait AppDriver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action);

    /// Called when the app is resumed. This happens after masonry handles resume, so windows and surfaces should be initialized.
    /// This corresponds to the winit::application::ApplicationHandler::resumed method.
    fn resumed(&mut self, event_loop: &ActiveEventLoop, masonry_state: &mut MasonryState<'_>) {
    }

    /// Called when the app is suspended. This happens before masonry handles suspend, so windows and surfaces should be available still.
    /// This corresponds to the winit::application::ApplicationHandler::suspended method.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop, masonry_state: &mut MasonryState<'_>) {
    }

    /// Called when the app receives a window event. Return `true` if the event was handled and
    /// should not be processed by the default handler.
    /// This corresponds to the winit::application::ApplicationHandler::window_event method.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: &WinitWindowEvent,
        masonry_state: &mut MasonryState<'_>,
    ) -> bool {
        false
    }

    /// Called when the app receives a device event. Return `true` if the event was handled and
    /// should not be processed by the default handler.
    /// This corresponds to the winit::application::ApplicationHandler::device_event method.
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: &winit::event::DeviceEvent,
        masonry_state: &mut MasonryState<'_>,
    ) -> bool {
        false
    }

    /// Called when the app receives a accesskit event. Return `true` if the event was handled and
    /// should not be processed by the default handler.
    /// This corresponds to the winit::application::ApplicationHandler::user_event method.
    fn user_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &accesskit_winit::Event,
        masonry_state: &mut MasonryState<'_>,
    ) -> bool {
        false
    }

    /// This corresponds to the winit::application::ApplicationHandler::new_events method.
    fn new_events(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: winit::event::StartCause,
        masonry_state: &mut MasonryState<'_>,
    ) {
    }

    /// This corresponds to the winit::application::ApplicationHandler::exiting method.
    fn exiting(&mut self, event_loop: &ActiveEventLoop, masonry_state: &mut MasonryState<'_>) {
    }

    /// This corresponds to the winit::application::ApplicationHandler::memory_warning method.
    fn memory_warning(
        &mut self,
        event_loop: &ActiveEventLoop,
        masonry_state: &mut MasonryState<'_>,
    ) {
    }

    /// This corresponds to the winit::application::ApplicationHandler::about_to_wait method.   
    fn about_to_wait(
        &mut self,
        event_loop: &ActiveEventLoop,
        masonry_state: &mut MasonryState<'_>,
    ) {
    }
}

impl<'a> DriverCtx<'a> {
    /// Return a [`WidgetMut`] to the root widget.
    pub fn get_root<W: Widget>(&mut self) -> WidgetMut<'_, W> {
        self.main_root_widget.downcast()
    }
}
