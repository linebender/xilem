// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use winit::{dpi::LogicalSize, error::EventLoopError, window::Window};
use xilem::{
    view::{button, flex, portal, prose, Axis, FlexExt, FlexSpacer},
    Color, EventLoop, EventLoopBuilder, TextAlignment, WidgetView, Xilem,
};
use xilem_core::one_of::OneOf3;

#[derive(Clone)]
struct Status {
    code: u32,
    message: &'static str,
}

struct HttpCats {
    statuses: Vec<Status>,
    // The currently active code.
    selected_code: Option<u32>,
}

impl HttpCats {
    fn view(&mut self) -> impl WidgetView<HttpCats> {
        let left_column = portal(flex((
            prose("Status"),
            self.statuses
                .iter_mut()
                .map(Status::list_view)
                .collect::<Vec<_>>(),
        )));
        let info_area = if let Some(selected_code) = self.selected_code {
            if let Some(selected_status) =
                self.statuses.iter_mut().find(|it| it.code == selected_code)
            {
                OneOf3::A(selected_status.details_view())
            } else {
                OneOf3::B(
                    prose(format!(
                        "Status code {selected_code} selected, but this was not found."
                    ))
                    .alignment(TextAlignment::Middle)
                    .brush(Color::YELLOW),
                )
            }
        } else {
            OneOf3::C(
                prose("No selection yet made. Select an item from the sidebar to continue.")
                    .alignment(TextAlignment::Middle),
            )
        };

        flex((
            // Add padding to the top for Android. Still a horrible hack
            FlexSpacer::Fixed(40.),
            flex((left_column.flex(1.), info_area.flex(1.)))
                .direction(Axis::Horizontal)
                .must_fill_major_axis(true)
                .flex(1.),
        ))
        .must_fill_major_axis(true)
    }
}

impl Status {
    fn list_view(&mut self) -> impl WidgetView<HttpCats> {
        let code = self.code;
        flex((
            // TODO: Reduce allocations here?
            prose(self.code.to_string()),
            prose(self.message),
            FlexSpacer::Flex(1.),
            button("Select", move |state: &mut HttpCats| {
                state.selected_code = Some(code);
            }),
            FlexSpacer::Fixed(masonry::theme::SCROLLBAR_WIDTH),
        ))
        .direction(Axis::Horizontal)
    }

    fn details_view(&mut self) -> impl WidgetView<HttpCats> {
        flex((
            prose(format!("HTTP Status Code: {}", self.code)),
            prose(self.message).text_size(20.),
            prose(format!(
                "(Downloaded image from: https://http.cat/{})",
                self.code
            )),
            prose("Copyright ©️ https://http.cat"),
        ))
        .main_axis_alignment(xilem::view::MainAxisAlignment::Start)
        .must_fill_major_axis(true)
    }
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = HttpCats {
        statuses: Status::parse_file(),
        selected_code: None,
    };

    let app = Xilem::new(data, HttpCats::view);
    let min_window_size = LogicalSize::new(200., 200.);

    let window_attributes = Window::default_attributes()
        .with_title("HTTP cats")
        .with_resizable(true)
        .with_min_inner_size(min_window_size);

    app.run_windowed_in(event_loop, window_attributes)
}

impl Status {
    /// Parse the supported HTTP cats.
    fn parse_file() -> Vec<Self> {
        let mut lines = STATUS_CODES_CSV.lines();
        let first_line = lines.next();
        assert_eq!(first_line, Some("code,message"));
        lines.flat_map(Status::parse_single).collect()
    }

    fn parse_single(line: &'static str) -> Option<Self> {
        let (code, message) = line.split_once(',')?;
        Some(Self {
            code: code.parse().ok()?,
            message: message.trim(),
        })
    }
}

/// The status codes supported by <https://http.cat>, used under the MIT license.
/// Full details can be found in `xilem/resources/data/http_cats_status/README.md` from
/// the workspace root.
const STATUS_CODES_CSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/data/http_cats_status/status.csv",
));

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

// Boilerplate code for android: Identical across all applications

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
// We believe that there are no other declarations using this name in the compiled objects here
#[allow(unsafe_code)]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}

// TODO: This is a hack because of how we handle our examples in Cargo.toml
// Ideally, we change Cargo to be more sensible here?
#[cfg(target_os = "android")]
#[allow(dead_code)]
fn main() {
    unreachable!()
}
