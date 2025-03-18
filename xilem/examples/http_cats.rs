// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of Async web requests in Xilem to access the <https://http.cat/> API.
//! This also demonstrates image loading.

#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use std::sync::Arc;

use masonry::widgets::{Alignment, LineBreaking};
use vello::peniko::{Blob, Image};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::fork;
use xilem::core::one_of::OneOf3;
use xilem::view::{
    Axis, FlexSpacer, Padding, ZStackExt, button, flex, image, inline_prose, portal, prose,
    sized_box, spinner, split, worker, zstack,
};
use xilem::{EventLoop, EventLoopBuilder, TextAlignment, WidgetView, Xilem, palette};

/// The main state of the application.
struct HttpCats {
    statuses: Vec<Status>,
    // The currently active (http status) code.
    selected_code: Option<u32>,
}

#[derive(Debug)]
struct Status {
    code: u32,
    message: &'static str,
    image: ImageState,
}

#[derive(Debug)]
/// The state of the download of an image from a URL
enum ImageState {
    NotRequested,
    Pending,
    // Error,
    Available(Image),
}

impl HttpCats {
    fn view(&mut self) -> impl WidgetView<Self> + use<> {
        let left_column = sized_box(portal(flex((
            prose("Status"),
            self.statuses
                .iter_mut()
                .map(Status::list_view)
                .collect::<Vec<_>>(),
        ))))
        .padding(Padding::leading(5.));

        let (info_area, worker_value) = if let Some(selected_code) = self.selected_code {
            if let Some(selected_status) =
                self.statuses.iter_mut().find(|it| it.code == selected_code)
            {
                // If we haven't requested the image yet, make sure we do so.
                let value = match selected_status.image {
                    ImageState::NotRequested => {
                        // TODO: Should a view_function be editing `self`?
                        // This feels too imperative.
                        selected_status.image = ImageState::Pending;
                        Some(selected_code)
                    }
                    // If the image is pending, that means that worker already knows about it.
                    // We don't set the requested code to `selected_code` here because we could have been on
                    // a different view in-between, so we don't want to request the same image twice.
                    ImageState::Pending => None,
                    ImageState::Available(_) => None,
                };
                (OneOf3::A(selected_status.details_view()), value)
            } else {
                (
                    OneOf3::B(
                        prose(format!(
                            "Status code {selected_code} selected, but this was not found."
                        ))
                        .alignment(TextAlignment::Middle)
                        .brush(palette::css::YELLOW),
                    ),
                    None,
                )
            }
        } else {
            (
                OneOf3::C(
                    prose("No selection yet made. Select an item from the sidebar to continue.")
                        .alignment(TextAlignment::Middle),
                ),
                None,
            )
        };

        // TODO: Should `web_image` be a built-in component?

        fork(
            flex((
                // Add padding to the top for Android. Still a horrible hack
                FlexSpacer::Fixed(40.),
                split(left_column, portal(sized_box(info_area).expand_width())).split_point(0.4),
            ))
            .must_fill_major_axis(true),
            worker(
                worker_value,
                |proxy, mut rx| async move {
                    while let Some(request) = rx.recv().await {
                        if let Some(code) = request {
                            let proxy = proxy.clone();
                            tokio::task::spawn(async move {
                                let url = format!("https://http.cat/{code}");
                                let result = image_from_url(&url).await;
                                match result {
                                    // We choose not to handle the case where the event loop has ended
                                    Ok(image) => drop(proxy.message((code, image))),
                                    // TODO: Report in the frontend
                                    Err(err) => {
                                        tracing::warn!(
                                            "Loading image for HTTP status code {code} from {url} failed: {err:?}"
                                        );
                                    }
                                }
                            });
                        }
                    }
                },
                |state: &mut Self, (code, image): (u32, Image)| {
                    if let Some(status) = state.statuses.iter_mut().find(|it| it.code == code) {
                        status.image = ImageState::Available(image);
                    } else {
                        // TODO: Error handling?
                    }
                },
            ),
        )
    }
}

/// Load a [`vello::peniko::Image`] from the given url.
async fn image_from_url(url: &str) -> anyhow::Result<Image> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let image = image::load_from_memory(&bytes)?.into_rgba8();
    let width = image.width();
    let height = image.height();
    let data = image.into_vec();
    Ok(Image::new(
        Blob::new(Arc::new(data)),
        vello::peniko::ImageFormat::Rgba8,
        width,
        height,
    ))
}

impl Status {
    fn list_view(&mut self) -> impl WidgetView<HttpCats> + use<> {
        let code = self.code;
        flex((
            // TODO: Reduce allocations here?
            inline_prose(self.code.to_string()),
            inline_prose(self.message),
            FlexSpacer::Flex(1.),
            // TODO: Spinner if image pending?
            // TODO: Tick if image loaded?
            button("Select", move |state: &mut HttpCats| {
                state.selected_code = Some(code);
            }),
            FlexSpacer::Fixed(masonry::theme::SCROLLBAR_WIDTH),
        ))
        .direction(Axis::Horizontal)
    }

    fn details_view(&mut self) -> impl WidgetView<HttpCats> + use<> {
        let image = match &self.image {
            ImageState::NotRequested => OneOf3::A(
                prose("Failed to start fetching image. This is a bug!")
                    .alignment(TextAlignment::Middle),
            ),
            ImageState::Pending => OneOf3::B(sized_box(spinner()).width(80.).height(80.)),
            // TODO: Alt text?
            ImageState::Available(image_data) => {
                let attribution = sized_box(
                    sized_box(
                        prose("Copyright ©️ https://http.cat")
                            .line_break_mode(LineBreaking::Clip)
                            .alignment(TextAlignment::End),
                    )
                    .padding(4.)
                    .rounded(4.)
                    .background(palette::css::BLACK.multiply_alpha(0.5)),
                )
                .padding((30., 42., 0., 0.));
                OneOf3::C(zstack((
                    image(image_data),
                    attribution.alignment(Alignment::TopTrailing),
                )))
            }
        };
        flex((
            prose(format!("HTTP Status Code: {}", self.code)).alignment(TextAlignment::Middle),
            prose(self.message)
                .text_size(20.)
                .alignment(TextAlignment::Middle),
            FlexSpacer::Fixed(10.),
            image,
        ))
        .main_axis_alignment(xilem::view::MainAxisAlignment::Start)
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
        lines.flat_map(Self::parse_single).collect()
    }

    fn parse_single(line: &'static str) -> Option<Self> {
        let (code, message) = line.split_once(',')?;
        Some(Self {
            code: code.parse().ok()?,
            message: message.trim(),
            image: ImageState::NotRequested,
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
