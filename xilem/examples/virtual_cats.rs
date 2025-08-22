// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of virtual scrolling with Async web requests in Xilem
//!
//! Uses the same dataset as the `http_cats` example.

#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use std::sync::Arc;
use std::time::Duration;

use masonry::core::ArcStr;
use masonry::properties::types::{AsUnit, UnitPoint};
use masonry::properties::{LineBreaking, Padding};
use vello::peniko::{Blob, Image};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::core::fork;
use xilem::core::one_of::{OneOf, OneOf3};
use xilem::palette::css::{BLACK, WHITE};
use xilem::style::Style as _;
use xilem::view::{
    ObjectFit, ZStackExt, flex, image, label, prose, sized_box, spinner, virtual_scroll, zstack,
};
use xilem::{
    Color, EventLoop, EventLoopBuilder, FontWeight, TextAlign, WidgetView, WindowOptions, Xilem,
};

/// The main state of the application.
struct VirtualCats {
    statuses: Vec<Status>,
}

#[derive(Debug)]
struct Status {
    code: u32,
    message: ArcStr,
    image: ImageState,
}

#[derive(Debug)]
/// The state of the download of an image from a URL
enum ImageState {
    Pending,
    Available(Image),
    Error(anyhow::Error),
}

impl VirtualCats {
    fn virtual_item(&mut self, idx: i64) -> impl WidgetView<Self> + use<> {
        let index: usize = idx.try_into().expect("VirtualScroll bounds set correctly.");
        let item = self
            .statuses
            .get_mut(index)
            .expect("VirtualScroll bounds set correctly.");
        let spawn_task = match &item.image {
            ImageState::Pending | ImageState::Error(_) => true,
            ImageState::Available(_) => false,
        };
        let task = if spawn_task {
            // Capturing the code is valid here, because this will never change for this view.
            let code = item.code;
            // If the cat is not loaded yet, we create a task to load it.
            // N.B. If we wanted to batch requests, the lifecycle unfortunately gets a
            // lot more complicated currently.
            // Note that if this item scrolls offscreen, the ongoing load will be cancelled.
            Some(xilem::view::task_raw(
                move |proxy| {
                    async move {
                        let url = format!("https://http.cat/{code}");
                        // Add an artificial delay to show how variable sizes work
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        let result = image_from_url(&url).await;
                        drop(proxy.message(result));
                    }
                },
                move |state: &mut Self, message| {
                    let Some(status) = state.statuses.iter_mut().find(|it| it.code == code) else {
                        unreachable!("We never remove items from `statuses`")
                    };
                    status.image = match message {
                        Ok(image) => ImageState::Available(image),
                        Err(err) => ImageState::Error(err),
                    };
                },
            ))
        } else {
            None
        };
        let img = match &item.image {
            ImageState::Available(img) => {
                let attribution = sized_box(
                    sized_box(
                        prose("Copyright ©️ https://http.cat")
                            .line_break_mode(LineBreaking::Clip)
                            .text_alignment(TextAlign::End),
                    )
                    .padding(4.)
                    .corner_radius(4.)
                    .background_color(BLACK.multiply_alpha(0.5)),
                )
                .padding(Padding {
                    left: 0.,
                    right: 42.,
                    top: 30.,
                    bottom: 0.,
                });
                let imgview = zstack((
                    image(img).fit(ObjectFit::FitWidth),
                    attribution.alignment(UnitPoint::TOP_RIGHT),
                ));
                OneOf3::A(imgview)
            }
            ImageState::Pending => OneOf::B(sized_box(spinner()).width(80.px()).height(80.px())),
            ImageState::Error(err) => {
                // the people deserve their cat.
                // It is vital that the cat explains what went wrong.
                let emojicat = label("😿").text_size(48.);
                let errorstring = prose(err.to_string())
                    .text_size(18.0)
                    .text_color(Color::from_rgb8(0x85, 0, 0))
                    .weight(FontWeight::BOLD);

                let view = flex((errorstring, emojicat))
                    .background_color(WHITE)
                    .cross_axis_alignment(xilem::view::CrossAxisAlignment::Start)
                    .padding(16.0)
                    .corner_radius(8.0);
                OneOf::C(view)
            }
        };
        fork(flex((prose(item.message.clone()), img)), task)
    }

    fn view(&mut self) -> impl WidgetView<Self> + use<> {
        sized_box(virtual_scroll(
            0..self.statuses.len() as i64,
            Self::virtual_item,
        ))
        .padding(Padding::horizontal(10.0))
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

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = VirtualCats {
        statuses: Status::parse_file(),
    };

    let app = Xilem::new_simple(
        data,
        VirtualCats::view,
        WindowOptions::new("Virtualised HTTP cats")
            .with_min_inner_size(LogicalSize::new(200., 200.)),
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
            message: format!("{}:", message.trim()).into(),
            image: ImageState::Pending,
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

// TODO: This example doesn't support Android because the virtual scroll component doesn't have gesture support.
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::builder())
}
