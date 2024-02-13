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

use std::any::Any;

use accesskit::TreeUpdate;
use glazier::{
    Application, Cursor, HotKey, IdleToken, Menu, PointerEvent, Region, Scalable, SysMods,
    WinHandler, WindowBuilder, WindowHandle,
};
use vello::{
    kurbo::{Affine, Size},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};

use crate::{app::App, view::View, widget::Event};

// This is a bit of a hack just to get a window launched. The real version
// would deal with multiple windows and have other ways to configure things.
pub struct AppLauncher<T, V: View<T>> {
    title: String,
    app: App<T, V>,
}

// The logic of this struct is mostly parallel to DruidHandler in win_handler.rs.
struct MainState<T, V: View<T>> {
    handle: WindowHandle,
    app: App<T, V>,
    render_cx: RenderContext,
    surface: Option<RenderSurface>,
    renderer: Option<Renderer>,
    scene: Scene,
    counter: u64,
}

const QUIT_MENU_ID: u32 = 0x100;

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
        let glazier_app = Application::new().unwrap();
        let mut file_menu = Menu::new();
        file_menu.add_item(
            QUIT_MENU_ID,
            "E&xit",
            Some(&HotKey::new(SysMods::Cmd, "q")),
            Some(false),
            true,
        );
        let mut menubar = Menu::new();
        menubar.add_dropdown(Menu::new(), "Application", true);
        menubar.add_dropdown(file_menu, "&File", true);
        let _guard = self.app.rt.enter();
        let main_state = MainState::new(self.app);
        let window = WindowBuilder::new(glazier_app.clone())
            .handler(Box::new(main_state))
            .title(self.title)
            .menu(menubar)
            .size(Size::new(1024., 768.))
            .build()
            .unwrap();
        window.show();
        glazier_app.run(None);
    }
}

impl<T: Send + 'static, V: View<T> + 'static> WinHandler for MainState<T, V> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        self.app.connect(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, _: &Region) {
        self.app.paint();
        self.render();
        self.schedule_render();
    }

    fn idle(&mut self, _: IdleToken) {}

    fn command(&mut self, id: u32) {
        match id {
            QUIT_MENU_ID => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn accesskit_tree(&mut self) -> TreeUpdate {
        self.app.accesskit_connected = true;
        self.app.accessibility()
    }

    fn accesskit_action(&mut self, request: accesskit::ActionRequest) {
        self.app
            .window_event(Event::TargetedAccessibilityAction(request));
        self.handle.invalidate();
    }

    fn pointer_down(&mut self, event: &PointerEvent) {
        self.app.window_event(Event::MouseDown(event.into()));
        self.handle.invalidate();
    }

    fn pointer_up(&mut self, event: &PointerEvent) {
        self.app.window_event(Event::MouseUp(event.into()));
        self.handle.invalidate();
    }

    fn pointer_move(&mut self, event: &PointerEvent) {
        self.app.window_event(Event::MouseMove(event.into()));
        self.handle.invalidate();
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn wheel(&mut self, event: &PointerEvent) {
        self.app.window_event(Event::MouseWheel(event.into()));
        self.handle.invalidate();
    }

    fn pointer_leave(&mut self) {
        self.app.window_event(Event::MouseLeft());
        self.handle.invalidate();
    }

    fn size(&mut self, size: Size) {
        self.app.size(size);
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T, V: View<T>> MainState<T, V>
where
    T: Send,
{
    fn new(app: App<T, V>) -> Self {
        let render_cx = RenderContext::new().unwrap();
        MainState {
            handle: Default::default(),
            app,
            render_cx,
            surface: None,
            renderer: None,
            scene: Scene::default(),
            counter: 0,
        }
    }

    fn schedule_render(&self) {
        self.handle.invalidate();
    }

    fn render(&mut self) {
        let fragment = self.app.fragment();
        let handle = &self.handle;
        let scale = handle.get_scale().unwrap_or_default();
        let insets = handle.content_insets().to_px(scale);
        let mut size = handle.get_size().to_px(scale);
        size.width -= insets.x_value();
        size.height -= insets.y_value();
        let width = size.width as u32;
        let height = size.height as u32;
        if self.surface.is_none() {
            //println!("render size: {:?}", size);
            self.surface = Some(
                tokio::runtime::Handle::current()
                    .block_on(self.render_cx.create_surface(handle, width, height))
                    .unwrap(),
            );
        }
        if let Some(surface) = self.surface.as_mut() {
            if surface.config.width != width || surface.config.height != height {
                self.render_cx.resize_surface(surface, width, height);
            }
            let (scale_x, scale_y) = (scale.x(), scale.y());
            let transform = if scale_x != 1.0 || scale_y != 1.0 {
                Some(Affine::scale_non_uniform(scale_x, scale_y))
            } else {
                None
            };
            self.scene.reset();
            self.scene.append(fragment, transform);
            self.counter += 1;
            let surface_texture = surface
                .surface
                .get_current_texture()
                .expect("failed to acquire next swapchain texture");
            let dev_id = surface.dev_id;
            let device = &self.render_cx.devices[dev_id].device;
            let queue = &self.render_cx.devices[dev_id].queue;
            let renderer_options = RendererOptions {
                surface_format: Some(surface.format),
                use_cpu: false,
                antialiasing_support: AaSupport {
                    area: true,
                    msaa8: false,
                    msaa16: false,
                },
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
}
