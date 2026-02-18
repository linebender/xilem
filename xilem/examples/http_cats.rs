// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of Async web requests in Xilem to access the <https://http.cat/> API.
//! This also demonstrates image loading.

#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use std::sync::Arc;

use masonry::layout::{AsUnit, Length, UnitPoint};
use masonry::properties::{LineBreaking, Padding};
use tokio::sync::mpsc::UnboundedSender;
use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::core::fork;
use xilem::core::one_of::OneOf3;
use xilem::style::Style as _;
use xilem::view::{
    FlexExt, FlexSpacer, ZStackExt, flex_col, flex_row, image, inline_prose, portal, prose,
    sized_box, spinner, split, text_button, worker, zstack,
};
use xilem::{EventLoop, EventLoopBuilder, TextAlign, WidgetView, WindowOptions, Xilem, palette};

/// The main state of the application.
struct HttpCats {
    statuses: Vec<Status>,
    // The currently active (http status) code.
    selected_code: Option<u32>,
    /// Send a status code to download the image for it.
    download_sender: Option<UnboundedSender<u32>>,
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
    Available(ImageData),
}

impl HttpCats {
    fn view(&mut self) -> impl WidgetView<Self> + use<> {
        let left_column = flex_col((
            prose("Status"),
            self.statuses
                .iter_mut()
                .map(Status::list_view)
                .collect::<Vec<_>>(),
        ))
        .padding(Padding::left(5.));

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
                    .text_alignment(TextAlign::Center)
                    .text_color(palette::css::YELLOW),
                )
            }
        } else {
            OneOf3::C(
                prose("No selection yet made. Select an item from the sidebar to continue.")
                    .text_alignment(TextAlign::Center),
            )
        };

        // TODO: Should `web_image` be a built-in component?

        fork(
            flex_col((
                // Add padding to the top for Android. Still a horrible hack
                FlexSpacer::Fixed(40.px()),
                split(portal(left_column), info_area)
                    .split_point(0.4)
                    .flex(1.),
            )),
            worker(
                |proxy, mut rx| async move {
                    while let Some(code) = rx.recv().await {
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
                },
                |state: &mut Self, sender| {
                    state.download_sender = Some(sender);
                },
                |state: &mut Self, (code, image): (u32, ImageData)| {
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

/// Load the [`ImageData`] from the given url.
///
/// N.B. This is functionality shared with Placehero and `virtual_cats`.
async fn image_from_url(url: &str) -> anyhow::Result<ImageData> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let image = image::load_from_memory(&bytes)?.into_rgba8();
    let width = image.width();
    let height = image.height();
    let data = image.into_vec();
    Ok(ImageData {
        data: Blob::new(Arc::new(data)),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    })
}

impl Status {
    fn list_view(&mut self) -> impl WidgetView<HttpCats> + use<> {
        let code = self.code;
        flex_row((
            // TODO: Reduce allocations here?
            inline_prose(self.code.to_string()),
            inline_prose(self.message),
            FlexSpacer::Flex(1.),
            // TODO: Spinner if image pending?
            // TODO: Tick if image loaded?
            text_button("Select", move |state: &mut HttpCats| {
                let status = state
                    .statuses
                    .iter_mut()
                    .find(|it| it.code == code)
                    .unwrap();

                if matches!(status.image, ImageState::NotRequested) {
                    state.download_sender.as_ref().unwrap().send(code).unwrap();
                    status.image = ImageState::Pending;
                }

                state.selected_code = Some(code);
            }),
            FlexSpacer::Fixed(Length::px(masonry::theme::SCROLLBAR_WIDTH)),
        ))
    }

    fn details_view(&mut self) -> impl WidgetView<HttpCats> + use<> {
        let image = match &self.image {
            ImageState::NotRequested => OneOf3::A(
                prose("Failed to start fetching image. This is a bug!")
                    .text_alignment(TextAlign::Center),
            ),
            ImageState::Pending => OneOf3::B(spinner().dims(80.px())),
            // TODO: Alt text?
            ImageState::Available(image_data) => {
                let attribution = sized_box(
                    sized_box(
                        prose("Copyright ©️ https://http.cat")
                            .line_break_mode(LineBreaking::Clip)
                            .text_alignment(TextAlign::End),
                    )
                    .padding(4.)
                    .corner_radius(4.)
                    .background_color(palette::css::BLACK.multiply_alpha(0.5)),
                )
                .padding(Padding {
                    left: 0.,
                    right: 42.,
                    top: 30.,
                    bottom: 0.,
                });
                OneOf3::C(zstack((
                    image(image_data.clone()),
                    attribution.alignment(UnitPoint::TOP_RIGHT),
                )))
            }
        };
        flex_col((
            prose(format!("HTTP Status Code: {}", self.code)).text_alignment(TextAlign::Center),
            prose(self.message)
                .text_size(20.)
                .text_alignment(TextAlign::Center),
            FlexSpacer::Fixed(10.px()),
            image.flex(1.),
        ))
        .main_axis_alignment(xilem::view::MainAxisAlignment::Start)
    }
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = HttpCats {
        statuses: Status::parse_file(),
        selected_code: None,
        download_sender: None,
    };

    let app = Xilem::new_simple(
        data,
        HttpCats::view,
        WindowOptions::new("HTTP cats").with_min_inner_size(LogicalSize::new(200., 200.)),
    );

    app.run_in(event_loop)
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

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
