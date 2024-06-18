// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{app_driver::AppDriver, widget::RootWidget};
use xilem_core::MessageResult;

use crate::{ViewCtx, WidgetView};

pub struct MasonryDriver<State, Logic, View, ViewState> {
    pub(crate) state: State,
    pub(crate) logic: Logic,
    pub(crate) current_view: View,
    pub(crate) view_ctx: ViewCtx,
    pub(crate) view_state: ViewState,
    pub(crate) app_interface: Option<Box<dyn XilemToAppInterface<State>>>,
}

impl<State, Logic, View> AppDriver for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    fn on_action(
        &mut self,
        ctx: &mut masonry::app_driver::DriverCtx<'_>,
        widget_id: masonry::WidgetId,
        action: masonry::Action,
    ) {
        if let Some(id_path) = self.view_ctx.widget_map.get(&widget_id) {
            let message_result = self.current_view.message(
                &mut self.view_state,
                id_path.as_slice(),
                Box::new(action),
                &mut self.state,
            );
            let rebuild = match message_result {
                MessageResult::Action(()) => {
                    // It's not entirely clear what to do here
                    true
                }
                MessageResult::RequestRebuild => true,
                MessageResult::Nop => false,
                MessageResult::Stale(_) => {
                    tracing::info!("Discarding message");
                    false
                }
            };
            if rebuild {
                let next_view = (self.logic)(&mut self.state);
                let mut root = ctx.get_root::<RootWidget<View::Widget>>();

                self.view_ctx.view_tree_changed = false;
                next_view.rebuild(
                    &self.current_view,
                    &mut self.view_state,
                    &mut self.view_ctx,
                    root.get_element(),
                );
                if cfg!(debug_assertions) && !self.view_ctx.view_tree_changed {
                    tracing::debug!("Nothing changed as result of action");
                }
                self.current_view = next_view;
            }
        } else {
            eprintln!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
        }
    }

    // Note: the mem swap in all the methods below is necessary because we need a mutable reference to self and app_interface.
    // Better method would be good (could at least roll the logic into a single method).
    // One danger of the below method is if we ever had a recursive call to the driver, we would lose the app_interface.
    // However, that is not likely to ever happen because these calls are all driven by the event loop and the event loop
    // is not recursive.

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().resumed(event_loop, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
    
    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().suspended(event_loop, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
    
    fn window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, window_id: winit::window::WindowId, event: &winit::event::WindowEvent, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            let ret = app_interface.as_mut().unwrap().window_event(event_loop, window_id, event, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            ret
        }
        else {
            false
        }
    }
    
    fn device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, device_id: winit::event::DeviceId, event: &winit::event::DeviceEvent, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            let ret = app_interface.as_mut().unwrap().device_event(event_loop, device_id, event, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            ret
        }
        else {
            false
        }
    }
    
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: &accesskit_winit::Event, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            let ret = app_interface.as_mut().unwrap().user_event(event_loop, event, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            ret
        }
        else {
            false
        }
    }
    
    fn new_events(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, cause: winit::event::StartCause, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().new_events(event_loop, cause, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
    
    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().exiting(event_loop, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
    
    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().memory_warning(event_loop, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
    
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if self.app_interface.is_some() {
            let mut app_interface = None;
            std::mem::swap(&mut self.app_interface, &mut app_interface);
            app_interface.as_mut().unwrap().about_to_wait(event_loop, self, masonry_state);
            std::mem::swap(&mut self.app_interface, &mut app_interface);
        }
    }
}

pub trait AppToXilemInterface<State> {
    fn get_state(&mut self) -> &mut State;

    fn submit_window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, window_id: winit::window::WindowId, event: winit::event::WindowEvent);

    fn submit_device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, device_id: winit::event::DeviceId, event: winit::event::DeviceEvent);

    fn submit_user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, event: accesskit_winit::Event);
}

impl<State, Logic, View> AppToXilemInterface<State> for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>
{
    fn get_state(&mut self) -> &mut State {
        &mut self.state
    }

    fn submit_window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, window_id: winit::window::WindowId, event: winit::event::WindowEvent) {
        masonry_state.submit_window_event(event_loop, self, window_id, event);
    }

    fn submit_device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, device_id: winit::event::DeviceId, event: winit::event::DeviceEvent) {
        masonry_state.submit_device_event(event_loop, self, device_id, event);
    }

    fn submit_user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>, event: accesskit_winit::Event) {
        masonry_state.submit_user_event(event_loop, self, event);
    }
}


pub trait XilemToAppInterface<State> {
    fn resumed(&self,  _event_loop: &winit::event_loop::ActiveEventLoop, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn suspended(&self, _event_loop: &winit::event_loop::ActiveEventLoop, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn window_event(&self, _event_loop: &winit::event_loop::ActiveEventLoop, _window_id: winit::window::WindowId, _event: &winit::event::WindowEvent, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }
    fn device_event(&self, _event_loop: &winit::event_loop::ActiveEventLoop, _device_id: winit::event::DeviceId, _event: &winit::event::DeviceEvent, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }
    fn user_event(&self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: &accesskit_winit::Event, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }    
    fn new_events(&self, _event_loop: &winit::event_loop::ActiveEventLoop, _cause: winit::event::StartCause, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn exiting(&self, _event_loop: &winit::event_loop::ActiveEventLoop, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn memory_warning(&self, _event_loop: &winit::event_loop::ActiveEventLoop, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn about_to_wait(&self, _event_loop: &winit::event_loop::ActiveEventLoop, xilem_interface: &mut dyn AppToXilemInterface<State>, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
}