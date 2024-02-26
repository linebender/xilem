// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{num::NonZeroUsize, sync::Arc};

use glazier::{Modifiers, PointerButton};
use vello::{
    kurbo::{Affine, Point, Size},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{app::App, view::View, widget::Event, widget::PointerCrusher};

// This is a bit of a hack just to get a window launched. The real version
// would deal with multiple windows and have other ways to configure things.
pub struct AppLauncher<T, V: View<T>> {
    title: String,
    app: App<T, V>,
}

// The logic of this struct is mostly parallel to DruidHandler in win_handler.rs.
struct MainState<'a, T, V: View<T>> {
    window: Arc<Window>,
    app: App<T, V>,
    render_cx: RenderContext,
    surface: RenderSurface<'a>,
    renderer: Option<Renderer>,
    scene: Scene,
    counter: u64,
    main_pointer: PointerCrusher,
}

impl<T: Send + 'static, V: View<T> + 'static> AppLauncher<T, V> {
    pub fn new(app: App<T, V>) -> Self {
        AppLauncher {
            title: "Xilem app".into(),
            app,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        let _guard = self.app.rt.enter();
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize {
                width: 1024.,
                height: 768.,
            })
            .build(&event_loop)
            .unwrap();
        let mut main_state = MainState::new(self.app, window);

        event_loop
            .run(move |event, elwt| match event {
                winit::event::Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                winit::event::Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => main_state.paint(),
                winit::event::Event::WindowEvent {
                    event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                    ..
                } => main_state.size(Size {
                    width: width.into(),
                    height: height.into(),
                }),
                winit::event::Event::WindowEvent {
                    event: WindowEvent::ModifiersChanged(modifiers),
                    ..
                } => {
                    let mut m = Modifiers::empty();
                    let ms = modifiers.state();
                    if ms.contains(winit::keyboard::ModifiersState::SHIFT) {
                        m |= Modifiers::SHIFT;
                    }
                    if ms.contains(winit::keyboard::ModifiersState::CONTROL) {
                        m |= Modifiers::CONTROL;
                    }
                    if ms.contains(winit::keyboard::ModifiersState::SUPER) {
                        m |= Modifiers::SUPER;
                    }
                    if ms.contains(winit::keyboard::ModifiersState::ALT) {
                        m |= Modifiers::ALT;
                    }
                    main_state.mods(m);
                }
                winit::event::Event::WindowEvent {
                    event:
                        WindowEvent::CursorMoved {
                            position: winit::dpi::PhysicalPosition { x, y },
                            ..
                        },
                    ..
                } => main_state.pointer_move(Point { x, y }),
                winit::event::Event::WindowEvent {
                    event: WindowEvent::CursorLeft { .. },
                    ..
                } => main_state.pointer_leave(),
                winit::event::Event::WindowEvent {
                    event: WindowEvent::MouseInput { state, button, .. },
                    ..
                } => {
                    let b = match button {
                        winit::event::MouseButton::Left => PointerButton::Primary,
                        winit::event::MouseButton::Right => PointerButton::Secondary,
                        winit::event::MouseButton::Middle => PointerButton::Auxiliary,
                        winit::event::MouseButton::Back => PointerButton::X1,
                        winit::event::MouseButton::Forward => PointerButton::X2,
                        winit::event::MouseButton::Other(_) => PointerButton::None,
                    };
                    match state {
                        winit::event::ElementState::Pressed => main_state.pointer_down(b),
                        winit::event::ElementState::Released => main_state.pointer_up(b),
                    }
                }
                _ => (),
            })
            .unwrap();
    }
}

impl<'a, T, V: View<T> + 'static> MainState<'a, T, V>
where
    T: Send + 'static,
{
    fn new(app: App<T, V>, window: Window) -> Self {
        let mut render_cx = RenderContext::new().unwrap();
        let size = window.inner_size();
        let window = Arc::new(window);
        let surface = tokio::runtime::Handle::current()
            .block_on(render_cx.create_surface(window.clone(), size.width, size.height))
            .unwrap();
        MainState {
            window,
            app,
            render_cx,
            surface,
            renderer: None,
            scene: Scene::default(),
            counter: 0,
            main_pointer: PointerCrusher::new(),
        }
    }

    fn size(&mut self, size: Size) {
        self.app.size(size * 1.0 / self.window.scale_factor());
    }

    fn mods(&mut self, mods: Modifiers) {
        self.main_pointer.mods(mods);
    }

    fn pointer_move(&mut self, pos: Point) {
        let scale_coefficient = 1.0 / self.window.scale_factor();
        self.app
            .window_event(Event::MouseMove(self.main_pointer.moved(Point {
                x: pos.x * scale_coefficient,
                y: pos.y * scale_coefficient,
            })));
        self.window.request_redraw();
    }

    fn pointer_down(&mut self, button: PointerButton) {
        self.app
            .window_event(Event::MouseDown(self.main_pointer.pressed(button)));
        self.window.request_redraw();
    }

    fn pointer_up(&mut self, button: PointerButton) {
        self.app
            .window_event(Event::MouseUp(self.main_pointer.released(button)));
        self.window.request_redraw();
    }

    fn pointer_leave(&mut self) {
        self.app.window_event(Event::MouseLeft());
        self.window.request_redraw();
    }

    fn paint(&mut self) {
        self.app.paint();
        self.render();
    }

    fn render(&mut self) {
        let fragment = self.app.fragment();
        let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width;
        let height = size.height;

        if self.surface.config.width != width || self.surface.config.height != height {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
        }
        let transform = if scale != 1.0 {
            Some(Affine::scale(scale))
        } else {
            None
        };
        self.scene.reset();
        self.scene.append(fragment, transform);
        self.counter += 1;

        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let dev_id = self.surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(self.surface.format),
            use_cpu: false,
            antialiasing_support: AaSupport {
                area: true,
                msaa8: false,
                msaa16: false,
            },
            num_init_threads: NonZeroUsize::new(1),
        };
        let render_params = RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .get_or_insert_with(|| Renderer::new(device, renderer_options).unwrap())
            .render_to_surface(device, queue, &self.scene, &surface_texture, &render_params)
            .expect("failed to render to surface");
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);
    }
}
