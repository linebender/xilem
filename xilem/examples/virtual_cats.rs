// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of virtual scrolling with Async web requests in Xilem
//!
//! Uses the same dataset as the `http_cats` example.

#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use masonry::core::ArcStr;
use masonry::widgets::Alignment;
use tokio::sync::mpsc::Sender;
use vello::peniko::{Blob, Image};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::core::fork;
use xilem::view::{
    ObjectFit, ZStackExt, flex, image, prose, sized_box, spinner, task_raw, virtual_scroll, zstack,
};
use xilem::{EventLoop, EventLoopBuilder, LineBreaking, TextAlignment, WidgetView, Xilem, palette};
use xilem_core::one_of::Either;

/// The main state of the application.
struct VirtualCats {
    statuses: Vec<Status>,
    tx: Option<Sender<u32>>,
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
    NotRequested,
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
            ImageState::NotRequested => {
                // We can't store the items to be scheduled in `self`, because the parent view isn't re-run when this function is executed
                // This is for the reasonable reason that that could create a loop, if this weren't conditional.
                // However, we probably should have an escape hatch for when this is conditional.
                self.tx.as_ref().unwrap().blocking_send(item.code).unwrap();
                item.image = ImageState::Pending;
                None
            }
            ImageState::Pending => None,
            ImageState::Available(image) => Some(image),
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
        flex((prose(item.message.clone()), img))
    }

    fn view(&mut self) -> impl WidgetView<Self> + use<> {
        let rx = if self.tx.is_none() {
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            self.tx = Some(tx);
            Some(Mutex::new(Some(rx)))
        } else {
            None
        };
        fork(
            virtual_scroll(0..self.statuses.len() as i64, Self::virtual_item),
            // TODO: This is really awful...
            // Ultimately, this doesn't really make sense as a view?
            task_raw(
                move |proxy| {
                    let mut rx = rx.as_ref().unwrap().lock().unwrap().take().unwrap();
                    async move {
                        while let Some(code) = rx.recv().await {
                            let proxy = proxy.clone();
                            tokio::task::spawn(async move {
                                let url = format!("https://http.cat/{code}");
                                // Add an artificial delay to show how variable sizes work
                                tokio::time::sleep(Duration::from_millis(300)).await;
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

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = VirtualCats {
        statuses: Status::parse_file(),
        tx: None,
    };

    let app = Xilem::new(data, VirtualCats::view);
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
            message: format!("{}:", message.trim()).into(),
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

// TODO: This example doesn't support Android because the virtual scroll component doesn't have gesture support.
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
