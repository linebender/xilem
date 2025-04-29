// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of virtual scrolling with Async web requests in Xilem
//!
//! Uses the same dataset as the `http_cats` example.

#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use std::sync::Arc;
use std::time::Duration;

use masonry::core::ArcStr;
use masonry::widgets::Alignment;
use vello::peniko::{Blob, Image};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::fork;
use xilem::view::{
    ObjectFit, Padding, ZStackExt, flex, image, prose, sized_box, spinner, virtual_scroll, zstack,
};
use xilem::{EventLoop, EventLoopBuilder, LineBreaking, TextAlignment, WidgetView, Xilem, palette};
use xilem_core::one_of::Either;

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
    // Error(?),
    Available(Image),
}

impl VirtualCats {
    fn virtual_item(&mut self, idx: i64) -> impl WidgetView<Self> + use<> {
        let index: usize = idx.try_into().expect("VirtualScroll bounds set correctly.");
        let item = self
            .statuses
            .get_mut(index)
            .expect("VirtualScroll bounds set correctly.");
        let img = match &item.image {
            ImageState::Pending => None,
            ImageState::Available(image) => Some(image),
        };
        let task = if img.is_none() {
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
                        match result {
                            // We choose not to handle the case where the event loop has ended
                            Ok(image) => drop(proxy.message(image)),
                            // TODO: Report in the frontend
                            Err(err) => {
                                tracing::warn!(
                                    "Loading image for HTTP status code {code} from {url} failed: {err:?}"
                                );
                            }
                        }
                    }
                },
                move |state: &mut Self, image| {
                    if let Some(status) = state.statuses.iter_mut().find(|it| it.code == code) {
                        status.image = ImageState::Available(image);
                    } else {
                        unreachable!("We never remove items from `statuses`")
                    }
                },
            ))
        } else {
            None
        };
        let img = if let Some(img) = img {
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
            Either::A(zstack((
                image(img).fit(ObjectFit::FitWidth),
                attribution.alignment(Alignment::TopTrailing),
            )))
        } else {
            Either::B(sized_box(spinner()).width(80.).height(80.))
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

    let app = Xilem::new(data, VirtualCats::view);
    let min_window_size = LogicalSize::new(200., 200.);

    let window_attributes = Window::default_attributes()
        .with_title("Virtualised HTTP cats")
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
    run(EventLoop::with_user_event())
}
