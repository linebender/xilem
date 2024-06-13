// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{app_driver::AppDriver, widget::RootWidget};
use xilem_core::MessageResult;

use crate::{ViewCtx, WidgetView};

pub struct MasonryDriver<State, Logic, View, ViewState> {
    pub(crate) state: State,
    pub(crate) logic: Logic,
    pub(crate) current_view: View,
    pub(crate) view_cx: ViewCtx,
    pub(crate) view_state: ViewState,
    pub(crate) masonry_interface: Option<Box<dyn MasonryInterface<State>>>,
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
        if let Some(id_path) = self.view_cx.widget_map.get(&widget_id) {
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

                self.view_cx.view_tree_changed = false;
                next_view.rebuild(
                    &self.current_view,
                    &mut self.view_state,
                    &mut self.view_cx,
                    root.get_element(),
                );
                if cfg!(debug_assertions) && !self.view_cx.view_tree_changed {
                    tracing::debug!("Nothing changed as result of action");
                }
                self.current_view = next_view;
            }
        } else {
            eprintln!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
        }
    }
    
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.resumed(&mut self.state, event_loop, masonry_state);
        }
    }
    
    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.suspended(&mut self.state, event_loop, masonry_state);
        }
    }
    
    fn window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, window_id: winit::window::WindowId, event: &winit::event::WindowEvent, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.window_event(&mut self.state, event_loop, window_id, event, masonry_state)
        } else {
            false
        }
    }
    
    fn device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, device_id: winit::event::DeviceId, event: &winit::event::DeviceEvent, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.device_event(&mut self.state, event_loop, device_id, event, masonry_state)
        } else {
            false
        }
    }
    
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: &accesskit_winit::Event, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.user_event(&mut self.state, event_loop, event, masonry_state)
        } else {
            false
        }
    }
    
    fn new_events(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, cause: winit::event::StartCause, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.new_events(&mut self.state, event_loop, cause, masonry_state);
        }
    }
    
    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.exiting(&mut self.state, event_loop, masonry_state);
        }
    }
    
    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.memory_warning(&mut self.state, event_loop, masonry_state);
        }
    }
    
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {
        if let Some(winit_interface) = &self.masonry_interface {
            winit_interface.about_to_wait(&mut self.state, event_loop, masonry_state);
        }
    }
}

pub trait MasonryInterface<State> {
    fn resumed(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn suspended(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}    
    fn window_event(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _window_id: winit::window::WindowId, _event: &winit::event::WindowEvent, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }
    fn device_event(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _device_id: winit::event::DeviceId, _event: &winit::event::DeviceEvent, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }
    fn user_event(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _event: &accesskit_winit::Event, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) -> bool { false }    
    fn new_events(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _cause: winit::event::StartCause, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn exiting(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn memory_warning(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
    fn about_to_wait(&self, _state: &mut State, _event_loop: &winit::event_loop::ActiveEventLoop, _masonry_state: &mut masonry::event_loop_runner::MasonryState<'_>) {}
}