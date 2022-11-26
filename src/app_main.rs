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

use glazier::{
    kurbo::{Affine, Size},
    Application, Cursor, HotKey, IdleToken, Menu, MouseEvent, Region, Scalable, SysMods,
    WinHandler, WindowBuilder, WindowHandle,
};
use parley::FontContext;
use piet_scene::{Scene, SceneBuilder, SceneFragment};

use crate::{app::App, widget::RawEvent, View, Widget};

// This is a bit of a hack just to get a window launched. The real version
// would deal with multiple windows and have other ways to configure things.
pub struct AppLauncher<T, V: View<T>> {
    title: String,
    app: App<T, V>,
}

// The logic of this struct is mostly parallel to DruidHandler in win_handler.rs.
struct MainState<T, V: View<T>>
where
    V::Element: Widget,
{
    handle: WindowHandle,
    app: App<T, V>,
    pgpu_state: Option<crate::render::PgpuState>,
    font_context: FontContext,
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
        let mut file_menu = Menu::new();
        file_menu.add_item(
            QUIT_MENU_ID,
            "E&xit",
            Some(&HotKey::new(SysMods::Cmd, "q")),
            Some(true),
            false,
        );
        let mut menubar = Menu::new();
        menubar.add_dropdown(Menu::new(), "Application", true);
        menubar.add_dropdown(file_menu, "&File", true);
        let druid_app = Application::new().unwrap();
        let mut builder = WindowBuilder::new(druid_app.clone());
        let _guard = self.app.rt.enter();
        let main_state = MainState::new(self.app);
        builder.set_handler(Box::new(main_state));
        builder.set_title(self.title);
        builder.set_menu(menubar);
        builder.set_size(Size::new(1024., 768.));
        let window = builder.build().unwrap();
        window.show();
        druid_app.run(None);
    }
}

impl<T: Send + 'static, V: View<T> + 'static> WinHandler for MainState<T, V>
where
    V::Element: Widget,
{
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

    // TODO: temporary hack
    fn idle(&mut self, _: IdleToken) {
        self.app.paint();
        self.render();
        self.schedule_render();
    }

    fn command(&mut self, id: u32) {
        match id {
            QUIT_MENU_ID => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseDown(event.into()));
        self.handle.invalidate();
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseUp(event.into()));
        self.handle.invalidate();
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseMove(event.into()));
        self.handle.invalidate();
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseWheel(event.into()));
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
    V::Element: Widget,
    T: Send,
{
    fn new(app: App<T, V>) -> Self {
        let state = MainState {
            handle: Default::default(),
            app,
            font_context: FontContext::new(),
            pgpu_state: None,
            scene: Scene::default(),
            counter: 0,
        };
        state
    }

    #[cfg(target_os = "macos")]
    fn schedule_render(&self) {
        self.handle
            .get_idle_handle()
            .unwrap()
            .schedule_idle(IdleToken::new(0));
    }

    #[cfg(not(target_os = "macos"))]
    fn schedule_render(&self) {
        self.handle.invalidate();
    }

    fn render(&mut self) {
        let fragment = self.app.fragment();
        if self.pgpu_state.is_none() {
            let handle = &self.handle;
            let scale = handle.get_scale().unwrap();
            let insets = handle.content_insets().to_px(scale);
            let mut size = handle.get_size().to_px(scale);
            size.width -= insets.x_value();
            size.height -= insets.y_value();
            println!("render size: {:?}", size);
            self.pgpu_state = Some(
                crate::render::PgpuState::new(
                    handle,
                    handle,
                    size.width as usize,
                    size.height as usize,
                )
                .unwrap(),
            );
        }
        if let Some(pgpu_state) = self.pgpu_state.as_mut() {
            let scale = self.handle.get_scale().unwrap_or_default();
            let (scale_x, scale_y) = (scale.x(), scale.y());
            let transform = if scale_x != 1.0 || scale_y != 1.0 {
                Some(Affine::scale_non_uniform(scale_x, scale_y))
            } else {
                None
            };
            if let Some(_timestamps) = pgpu_state.pre_render() {}
            let mut builder = SceneBuilder::for_scene(&mut self.scene);
            builder.append(&fragment, transform);
            //crate::test_scenes::render(&mut self.font_context, &mut self.scene, 0, self.counter);
            self.counter += 1;
            pgpu_state.render(&self.scene);
        }
    }
}
